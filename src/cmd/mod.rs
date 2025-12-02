use crate::connection::Connection;
use crate::db::Db;
use crate::frame::Frame;
use bytes::Bytes;
use std::io;
use std::time::{Duration, Instant};

/// Represents a Redis command
pub enum Command {
    /// PING [message] - Test connection
    Ping(Option<Bytes>),

    /// SET key value [EX seconds] - Set a key-value pair with optional expiration
    Set {
        key: String,
        value: Bytes,
        expires_at: Option<Instant>,
    },

    /// GET key - Get a value by key
    Get { key: String },

    /// ECHO message - Echo back a message
    Echo { message: Bytes },

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
            Frame::Bulk(data) => std::str::from_utf8(data)
                .map_err(|_| "invalid UTF-8 in command name")?
                .to_uppercase(),
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
            "SET" => {
                // SET key value [EX seconds]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'set' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SET key must be a string".to_string()),
                };

                let value = match &array[2] {
                    Frame::Bulk(data) => data.clone(),
                    Frame::Simple(s) => Bytes::from(s.clone()),
                    _ => return Err("SET value must be a string".to_string()),
                };

                // Parse optional EX (expiration in seconds)
                let mut expires_at = None;
                let mut i = 3;
                while i < array.len() {
                    let option = match &array[i] {
                        Frame::Bulk(data) => std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in option")?
                            .to_uppercase(),
                        Frame::Simple(s) => s.to_uppercase(),
                        _ => return Err("SET option must be a string".to_string()),
                    };

                    match option.as_str() {
                        "EX" => {
                            if i + 1 >= array.len() {
                                return Err("ERR syntax error".to_string());
                            }
                            let seconds = match &array[i + 1] {
                                Frame::Bulk(data) => {
                                    let s = std::str::from_utf8(data)
                                        .map_err(|_| "invalid UTF-8 in seconds")?;
                                    s.parse::<u64>().map_err(|_| {
                                        "ERR value is not an integer or out of range"
                                    })?
                                }
                                Frame::Simple(s) => s
                                    .parse::<u64>()
                                    .map_err(|_| "ERR value is not an integer or out of range")?,
                                _ => {
                                    return Err(
                                        "ERR value is not an integer or out of range".to_string()
                                    )
                                }
                            };
                            expires_at = Some(Instant::now() + Duration::from_secs(seconds));
                            i += 2;
                        }
                        _ => return Err(format!("ERR syntax error near '{}'", option)),
                    }
                }

                Ok(Command::Set {
                    key,
                    value,
                    expires_at,
                })
            }
            "GET" => {
                // GET key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'get' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("GET key must be a string".to_string()),
                };

                Ok(Command::Get { key })
            }
            "ECHO" => {
                // ECHO message
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'echo' command".to_string());
                }

                let message = match array.remove(1) {
                    Frame::Bulk(data) => data,
                    Frame::Simple(s) => Bytes::from(s),
                    _ => return Err("ECHO message must be a string".to_string()),
                };

                Ok(Command::Echo { message })
            }
            _ => Ok(Command::Unknown(cmd_name)),
        }
    }

    /// Execute the command and write the response to the connection
    pub async fn execute(&self, db: &Db, dst: &mut Connection) -> Result<(), io::Error> {
        match self {
            Command::Ping(msg) => {
                let response = if let Some(msg) = msg {
                    Frame::Bulk(msg.clone())
                } else {
                    Frame::Simple("PONG".to_string())
                };
                dst.write_frame(&response).await?;
            }
            Command::Set {
                key,
                value,
                expires_at,
            } => {
                // Write to database with optional expiration
                db.write_entry_with_expiration(key.clone(), value.clone(), *expires_at);

                // Return OK
                let response = Frame::Simple("OK".to_string());
                dst.write_frame(&response).await?;
            }
            Command::Get { key } => {
                // Read from database
                let response = if let Some(value) = db.read_entry(key) {
                    Frame::Bulk(value)
                } else {
                    Frame::Null
                };
                dst.write_frame(&response).await?;
            }
            Command::Echo { message } => {
                // Echo back the message
                let response = Frame::Bulk(message.clone());
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
