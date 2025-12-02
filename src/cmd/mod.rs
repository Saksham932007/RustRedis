use crate::connection::Connection;
use crate::db::Db;
use crate::frame::Frame;
use std::io;

/// Represents a Redis command
pub enum Command {
    /// Placeholder - will add specific commands soon
    Unknown(String),
}

impl Command {
    /// Parse a command from a frame
    pub fn from_frame(frame: Frame) -> Result<Command, String> {
        // Commands are sent as arrays: [command_name, arg1, arg2, ...]
        let array = match frame {
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
        
        // For now, all commands are unknown - we'll implement specific ones next
        Ok(Command::Unknown(cmd_name))
    }
    
    /// Execute the command and write the response to the connection
    pub async fn execute(&self, _db: &Db, dst: &mut Connection) -> Result<(), io::Error> {
        match self {
            Command::Unknown(cmd) => {
                let error = Frame::error(format!("ERR unknown command '{}'", cmd));
                dst.write_frame(&error).await?;
            }
        }
        Ok(())
    }
}
