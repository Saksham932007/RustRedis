use anyhow::Result;
use rust_redis::{cmd::Command, connection::Connection, db::Db, persistence::{Aof, AofSyncPolicy}};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    // Create the shared database
    let db = Db::new();

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
                            // Execute command silently to restore state
                            // We create a dummy connection for this
                            // In production, you'd want a better approach
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

                // Clone the db handle for this connection
                let db = db.clone();
                let aof = aof.clone();

                // Spawn a new task to handle the connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, db, aof).await {
                        error!("Error handling connection: {}", e);
                    }
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
async fn handle_connection(socket: TcpStream, db: Db, aof: Option<Arc<Aof>>) -> Result<()> {
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

        // Log write commands to AOF
        if let Some(ref aof_writer) = aof {
            if command.is_write_command() {
                if let Err(e) = aof_writer.append(&frame) {
                    error!("Failed to append to AOF: {}", e);
                }
            }
        }

        // Execute the command
        command.execute(&db, &mut connection).await?;
    }
}
