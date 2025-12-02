use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{info, error, debug};
use anyhow::Result;
use rust_redis::{connection::Connection, db::Db, cmd::Command};

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
                
                // Spawn a new task to handle the connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, db).await {
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
async fn handle_connection(socket: TcpStream, db: Db) -> Result<()> {
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
        let command = match Command::from_frame(frame) {
            Ok(cmd) => cmd,
            Err(e) => {
                error!("Failed to parse command: {}", e);
                continue;
            }
        };
        
        // Execute the command
        command.execute(&db, &mut connection).await?;
    }
}
