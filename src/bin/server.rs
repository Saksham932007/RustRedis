use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Bind the TCP listener to port 6379 (Redis default port)
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    
    println!("RustRedis server listening on 127.0.0.1:6379");
    println!("Press CTRL+C to shutdown gracefully");

    loop {
        tokio::select! {
            // Accept incoming connections
            result = listener.accept() => {
                let (socket, addr) = result?;
                
                println!("Accepted connection from: {}", addr);
                
                // Spawn a new task to handle the connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
                        eprintln!("Error handling connection: {}", e);
                    }
                });
            }
            
            // Listen for shutdown signal (CTRL+C)
            _ = signal::ctrl_c() => {
                println!("\nReceived shutdown signal. Gracefully shutting down...");
                break;
            }
        }
    }
    
    println!("Server shut down successfully");
    Ok(())
}

/// Handle a single client connection
async fn handle_connection(socket: TcpStream) -> Result<()> {
    // For now, just keep the connection alive
    // We'll implement the actual protocol handling in later commits
    let peer_addr = socket.peer_addr()?;
    println!("Handling connection from {}", peer_addr);
    
    // Keep the socket alive but don't do anything yet
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    
    Ok(())
}
