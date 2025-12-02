use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Bind the TCP listener to port 6379 (Redis default port)
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    
    println!("RustRedis server listening on 127.0.0.1:6379");

    loop {
        // Accept incoming connections
        let (socket, addr) = listener.accept().await?;
        
        println!("Accepted connection from: {}", addr);
        
        // Spawn a new task to handle the connection
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
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
