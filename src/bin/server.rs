use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{info, error, debug};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();
    
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
                
                // Spawn a new task to handle the connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
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
async fn handle_connection(socket: TcpStream) -> Result<()> {
    // For now, just keep the connection alive
    // We'll implement the actual protocol handling in later commits
    let peer_addr = socket.peer_addr()?;
    debug!("Handling connection from {}", peer_addr);
    
    // Keep the socket alive but don't do anything yet
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    
    Ok(())
}
