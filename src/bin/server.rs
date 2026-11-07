use anyhow::Result;
use rust_redis::{
    cmd::Command,
    command_metrics::{self, CommandMetricsCollector, MetricsStrategy, SharedCommandMetrics},
    connection::Connection,
    db::Db,
    db_dashmap::DbDashMap,
    frame::Frame,
    metrics::{Metrics, SharedMetrics},
    persistence::{Aof, AofSyncPolicy},
    pubsub::PubSub,
    store::Store,
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

    // Select the keyspace backend. The single-mutex store is the default so that
    // published results stay reproducible; the sharded store exists to test
    // whether keyspace lock contention confounds the metrics comparison.
    let store = std::env::var("RUSTREDIS_STORE").unwrap_or_else(|_| "mutex".to_string());
    match store.to_ascii_lowercase().as_str() {
        "mutex" => {
            info!("Keyspace store: mutex");
            serve::<Db>().await
        }
        "dashmap" => {
            info!("Keyspace store: dashmap");
            serve::<DbDashMap>().await
        }
        other => anyhow::bail!(
            "unknown RUSTREDIS_STORE '{}' (expected 'mutex' or 'dashmap')",
            other
        ),
    }
}

/// Run the server on the given keyspace backend
async fn serve<S: Store>() -> Result<()> {
    // Create the shared database
    let db = S::new();

    // Create Pub/Sub manager
    let pubsub = PubSub::new();
    info!("Pub/Sub system initialized");

    // Create metrics
    let metrics = Metrics::new();
    info!("Metrics system initialized");

    let disable_aof = std::env::var("RUSTREDIS_DISABLE_AOF")
        .map(|v| {
            let normalized = v.to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false);
    let aof_path =
        std::env::var("RUSTREDIS_AOF_PATH").unwrap_or_else(|_| "appendonly.aof".to_string());

    // Create per-command metrics collector
    let strategy = match std::env::var("RUSTREDIS_METRICS_STRATEGY") {
        Ok(name) => MetricsStrategy::parse(&name)
            .ok_or_else(|| anyhow::anyhow!("unknown RUSTREDIS_METRICS_STRATEGY '{}'", name))?,
        Err(_) => MetricsStrategy::Sharded2Key,
    };
    let command_metrics = CommandMetricsCollector::new(strategy);
    info!(
        "Command metrics initialized (strategy: {})",
        strategy.name()
    );

    // Start background flush task for ThreadLocalBatched strategy
    if let Some(tl_collector) = command_metrics.thread_local_collector() {
        command_metrics::start_thread_local_flush_task(tl_collector);
        info!("Thread-local metrics flush task started (100ms interval)");
    }

    // Start background flush task for ThreadLocalOwned strategy. This must run
    // on the same 100ms cadence as the HdrHistogram task, since the two
    // strategies are compared against each other to price the histogram payload
    // (docs/paper_design_axes.md): a difference in flush cadence would show up
    // as a difference in payload cost.
    if let Some(tlo_collector) = command_metrics.thread_local_owned_collector() {
        command_metrics::start_thread_local_owned_flush_task(tlo_collector);
        info!("Thread-local (owned-key) metrics flush task started (100ms interval)");
    }

    // Start background flush task for HdrHistogram strategy
    if let Some(hdr_collector) = command_metrics.hdr_histogram_collector() {
        command_metrics::start_hdr_flush_task(hdr_collector);
        info!("HdrHistogram metrics flush task started (100ms interval)");
    }

    // Initialize AOF persistence unless explicitly disabled for experiment runs.
    let aof = if disable_aof {
        warn!("AOF persistence disabled via RUSTREDIS_DISABLE_AOF");
        None
    } else {
        match Aof::new(&aof_path, AofSyncPolicy::EverySecond) {
            Ok(aof) => {
                info!(
                    "AOF persistence enabled with EverySecond sync policy (path: {})",
                    aof_path
                );
                let aof = Arc::new(aof);

                // Start background sync task
                Arc::clone(&aof).start_background_sync();

                // Try to load existing AOF file
                match Aof::load(&aof_path) {
                    Ok(frames) => {
                        info!("Loaded {} commands from AOF", frames.len());
                        // Replay commands to restore state
                        for frame in frames {
                            if let Ok(cmd) = Command::from_frame(&frame) {
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
        }
    };

    // Bind the TCP listener (defaults to the Redis port on loopback)
    let bind_addr =
        std::env::var("RUSTREDIS_BIND").unwrap_or_else(|_| "127.0.0.1:6379".to_string());
    let listener = TcpListener::bind(&bind_addr).await?;

    info!("RustRedis server listening on {}", listener.local_addr()?);
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
async fn handle_connection<S: Store>(
    socket: TcpStream,
    db: S,
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
        let frame = match connection.read_frame().await {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                // Connection closed
                debug!("Client disconnected");
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                // Malformed input: tell the client why, then drop the connection
                warn!("Protocol error: {}", e);
                let reply = Frame::error(format!("ERR Protocol error: {}", e));
                let _ = connection.write_frame(&reply).await;
                let _ = connection.flush().await;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        debug!("Received frame: {}", frame);

        // Parse the frame into a command
        let command = match Command::from_frame(&frame) {
            Ok(cmd) => cmd,
            Err(e) => {
                error!("Failed to parse command: {}", e);
                let message = if e.starts_with("ERR") {
                    e
                } else {
                    format!("ERR {}", e)
                };
                connection.write_frame(&Frame::error(message)).await?;
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
        let metrics_key_hint = command.metrics_key_hint();
        let cmd_start = Instant::now();
        command
            .execute(&db, &mut connection, &pubsub, &metrics, &command_metrics)
            .await?;
        let duration_us = cmd_start.elapsed().as_micros() as u64;
        metrics.add_command_duration_us(duration_us);
        metrics.increment_commands();

        // Record per-command metrics
        command_metrics.record(cmd_name, metrics_key_hint, duration_us);
    }
}
