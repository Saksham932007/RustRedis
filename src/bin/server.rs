use anyhow::Result;
use rust_redis::{
    cmd::Command,
    command_metrics::{self, CommandMetricsCollector, MetricsStrategy, SharedCommandMetrics},
    connection::Connection,
    db::Db,
    metrics::{Metrics, SharedMetrics},
    persistence::{Aof, AofSyncPolicy},
    pubsub::PubSub,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{debug, error, info, warn};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    // Create the shared database
    let db = Db::new();

    // Create Pub/Sub manager
    let pubsub = PubSub::new();
    info!("Pub/Sub system initialized");

    // Create metrics
    let metrics = Metrics::new();
    info!("Metrics system initialized");

    // Create per-command metrics collector
    let strategy = std::env::var("RUSTREDIS_METRICS_STRATEGY")
        .map(|s| MetricsStrategy::from_str_loose(&s))
        .unwrap_or(MetricsStrategy::Sharded);
    let command_metrics = CommandMetricsCollector::new(strategy);
    info!("Command metrics initialized (strategy: {})", strategy.name());

    // Start background flush task for ThreadLocalBatched strategy
    if let Some(tl_collector) = command_metrics.thread_local_collector() {
        command_metrics::start_flush_task(tl_collector);
        info!("Thread-local metrics flush task started (100ms interval)");
    }

    // Initialize AOF persistence
    let aof = match Aof::new("appendonly.aof", AofSyncPolicy::EverySecond) {
        Ok(aof) => {
            info!("AOF persistence enabled with EverySecond sync policy");
            let aof = Arc::new(aof);

            // Start background sync task
            Arc::clone(&aof).start_background_sync();

            // Try to load existing AOF file
            match Aof::load("appendonly.aof") {
                Ok(frames) => {
                    info!("Loaded {} commands from AOF", frames.len());
                    // Replay commands to restore state
                    for frame in frames {
                        if let Ok(cmd) = Command::from_frame(frame) {
                            let _ = cmd.replay(&db);
                        }
                    }
                    info!("AOF replay completed");
                }
                Err(e) => {
                    warn!("Could not load AOF (this is normal on first run): {}", e);
                }
            }

            Some(aof)
        }
        Err(e) => {
            warn!("AOF persistence disabled: {}", e);
            None
        }
    };

    // Bind the TCP listener to port 6379 (Redis default port)
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    info!("RustRedis server listening on 127.0.0.1:6379");
    info!("Press CTRL+C to shutdown gracefully");

    loop {
        tokio::select! {
            // Accept incoming connections
            result = listener.accept() => {
                let (socket, addr) = result?;

                info!("Accepted connection from: {}", addr);

                // Clone handles for this connection
                let db = db.clone();
                let aof = aof.clone();
                let pubsub = pubsub.clone();
                let metrics = Arc::clone(&metrics);
                let command_metrics = Arc::clone(&command_metrics);

                metrics.increment_connections();

                // Spawn a new task to handle the connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(
                        socket, db, aof, pubsub,
                        Arc::clone(&metrics),
                        Arc::clone(&command_metrics),
                    ).await {
                        error!("Error handling connection: {}", e);
                    }
                    metrics.decrement_connections();
                });
            }

            // Listen for shutdown signal (CTRL+C)
            _ = signal::ctrl_c() => {
                info!("Received shutdown signal. Gracefully shutting down...");
                break;
            }
        }
    }

    info!("Server shut down successfully");
    Ok(())
}

/// Handle a single client connection
async fn handle_connection(
    socket: TcpStream,
    db: Db,
    aof: Option<Arc<Aof>>,
    pubsub: PubSub,
    metrics: SharedMetrics,
    command_metrics: SharedCommandMetrics,
) -> Result<()> {
    // Wrap the socket in our Connection struct
    let mut connection = Connection::new(socket);

    debug!("Connection handler started");

    // Process commands in a loop
    loop {
        // Read a frame from the connection
        let frame = match connection.read_frame().await? {
            Some(frame) => frame,
            None => {
                // Connection closed
                debug!("Client disconnected");
                return Ok(());
            }
        };

        debug!("Received frame: {}", frame);

        // Parse the frame into a command
        let command = match Command::from_frame(frame.clone()) {
            Ok(cmd) => cmd,
            Err(e) => {
                error!("Failed to parse command: {}", e);
                continue;
            }
        };

        // Log write commands to AOF (with timing)
        if let Some(ref aof_writer) = aof {
            if command.is_write_command() {
                let aof_start = Instant::now();
                if let Err(e) = aof_writer.append(&frame) {
                    error!("Failed to append to AOF: {}", e);
                }
                metrics.add_aof_write_time_us(aof_start.elapsed().as_micros() as u64);
            }
        }

        // Execute the command (with timing)
        let cmd_name = command.name();
        let cmd_start = Instant::now();
        command
            .execute(&db, &mut connection, &pubsub, &metrics, &command_metrics)
            .await?;
        let duration_us = cmd_start.elapsed().as_micros() as u64;
        metrics.add_command_duration_us(duration_us);
        metrics.increment_commands();

        // Record per-command metrics
        command_metrics.record(cmd_name, duration_us);
    }
}
