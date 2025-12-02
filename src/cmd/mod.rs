use crate::connection::Connection;
use crate::db::Db;
use crate::frame::Frame;
use bytes::Bytes;
use std::io;

/// Represents a Redis command
pub enum Command {
    /// PING [message] - Test connection
    Ping(Option<Bytes>),
    
    /// Unknown command
    Unknown(String),
}

impl Command {
    /// Parse a command from a frame
    pub fn from_frame(frame: Frame) -> Result<Command, String> {
        // Commands are sent as arrays: [command_name, arg1, arg2, ...]
        let mut array = match frame {
            Frame::Array(arr) => arr,
            _ => return Err("command must be an array".to_string()),
        };
        
        if array.is_empty() {
            return Err("empty command".to_string());
        }
        
        // Extract command name
        let cmd_name = match &array[0] {
            Frame::Bulk(data) => {
                std::str::from_utf8(data)
                    .map_err(|_| "invalid UTF-8 in command name")?
                    .to_uppercase()
            }
            Frame::Simple(s) => s.to_uppercase(),
            _ => return Err("command name must be a string".to_string()),
        };
        
        // Match specific commands
        match cmd_name.as_str() {
            "PING" => {
                // PING can optionally take a message argument
                if array.len() == 1 {
                    Ok(Command::Ping(None))
                } else if array.len() == 2 {
                    let message = match array.remove(1) {
                        Frame::Bulk(data) => data,
                        Frame::Simple(s) => Bytes::from(s),
                        _ => return Err("PING message must be a string".to_string()),
                    };
                    Ok(Command::Ping(Some(message)))
                } else {
                    Err("ERR wrong number of arguments for 'ping' command".to_string())
                }
            }
            _ => Ok(Command::Unknown(cmd_name)),
        }
    }
    
    /// Execute the command and write the response to the connection
    pub async fn execute(&self, _db: &Db, dst: &mut Connection) -> Result<(), io::Error> {
        match self {
            Command::Ping(msg) => {
                let response = if let Some(msg) = msg {
                    Frame::Bulk(msg.clone())
                } else {
                    Frame::Simple("PONG".to_string())
                };
                dst.write_frame(&response).await?;
            }
            Command::Unknown(cmd) => {
                let error = Frame::error(format!("ERR unknown command '{}'", cmd));
                dst.write_frame(&error).await?;
            }
        }
        Ok(())
    }
}
