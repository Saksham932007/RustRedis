use crate::connection::Connection;
use crate::db::Db;
use crate::frame::Frame;
use crate::pubsub::PubSub;
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

    /// DEL key [key ...] - Delete one or more keys
    Del { keys: Vec<String> },

    /// EXISTS key - Check if key exists
    Exists { key: String },

    /// TYPE key - Get the type of a value
    Type { key: String },

    /// DBSIZE - Get the number of keys in the database
    DbSize,

    /// FLUSHDB - Clear all keys from the database
    FlushDb,

    /// KEYS pattern - Get all keys matching a pattern
    Keys { pattern: String },

    // List commands
    /// LPUSH key value [value ...] - Push values to the left of a list
    LPush { key: String, values: Vec<Bytes> },

    /// RPUSH key value [value ...] - Push values to the right of a list
    RPush { key: String, values: Vec<Bytes> },

    /// LPOP key - Pop a value from the left of a list
    LPop { key: String },

    /// RPOP key - Pop a value from the right of a list
    RPop { key: String },

    /// LRANGE key start stop - Get a range of elements from a list
    LRange {
        key: String,
        start: isize,
        stop: isize,
    },

    /// LLEN key - Get the length of a list
    LLen { key: String },

    // Set commands
    /// SADD key member [member ...] - Add members to a set
    SAdd { key: String, members: Vec<String> },

    /// SREM key member [member ...] - Remove members from a set
    SRem { key: String, members: Vec<String> },

    /// SMEMBERS key - Get all members of a set
    SMembers { key: String },

    /// SISMEMBER key member - Check if a member exists in a set
    SIsMember { key: String, member: String },

    /// SCARD key - Get the cardinality (size) of a set
    SCard { key: String },

    // Hash commands
    /// HSET key field value - Set a field in a hash
    HSet {
        key: String,
        field: String,
        value: Bytes,
    },

    /// HGET key field - Get a field from a hash
    HGet { key: String, field: String },

    /// HGETALL key - Get all fields and values from a hash
    HGetAll { key: String },

    /// HDEL key field [field ...] - Delete fields from a hash
    HDel { key: String, fields: Vec<String> },

    /// HEXISTS key field - Check if a field exists in a hash
    HExists { key: String, field: String },

    /// HLEN key - Get the number of fields in a hash
    HLen { key: String },

    // Pub/Sub commands
    /// PUBLISH channel message - Publish a message to a channel
    Publish { channel: String, message: Bytes },

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
            "DEL" => {
                // DEL key [key ...]
                if array.len() < 2 {
                    return Err("ERR wrong number of arguments for 'del' command".to_string());
                }

                let mut keys = Vec::new();
                for i in 1..array.len() {
                    let key = match &array[i] {
                        Frame::Bulk(data) => std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in key")?
                            .to_string(),
                        Frame::Simple(s) => s.clone(),
                        _ => return Err("DEL key must be a string".to_string()),
                    };
                    keys.push(key);
                }

                Ok(Command::Del { keys })
            }
            "EXISTS" => {
                // EXISTS key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'exists' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("EXISTS key must be a string".to_string()),
                };

                Ok(Command::Exists { key })
            }
            "TYPE" => {
                // TYPE key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'type' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("TYPE key must be a string".to_string()),
                };

                Ok(Command::Type { key })
            }
            "DBSIZE" => {
                // DBSIZE
                if array.len() != 1 {
                    return Err("ERR wrong number of arguments for 'dbsize' command".to_string());
                }

                Ok(Command::DbSize)
            }
            "FLUSHDB" => {
                // FLUSHDB
                if array.len() != 1 {
                    return Err("ERR wrong number of arguments for 'flushdb' command".to_string());
                }

                Ok(Command::FlushDb)
            }
            "KEYS" => {
                // KEYS pattern
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'keys' command".to_string());
                }

                let pattern = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in pattern")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("KEYS pattern must be a string".to_string()),
                };

                Ok(Command::Keys { pattern })
            }
            "LPUSH" => {
                // LPUSH key value [value ...]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'lpush' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("LPUSH key must be a string".to_string()),
                };

                let mut values = Vec::new();
                for i in 2..array.len() {
                    let value = match &array[i] {
                        Frame::Bulk(data) => data.clone(),
                        Frame::Simple(s) => Bytes::from(s.clone()),
                        _ => return Err("LPUSH value must be a string".to_string()),
                    };
                    values.push(value);
                }

                Ok(Command::LPush { key, values })
            }
            "RPUSH" => {
                // RPUSH key value [value ...]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'rpush' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("RPUSH key must be a string".to_string()),
                };

                let mut values = Vec::new();
                for i in 2..array.len() {
                    let value = match &array[i] {
                        Frame::Bulk(data) => data.clone(),
                        Frame::Simple(s) => Bytes::from(s.clone()),
                        _ => return Err("RPUSH value must be a string".to_string()),
                    };
                    values.push(value);
                }

                Ok(Command::RPush { key, values })
            }
            "LPOP" => {
                // LPOP key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'lpop' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("LPOP key must be a string".to_string()),
                };

                Ok(Command::LPop { key })
            }
            "RPOP" => {
                // RPOP key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'rpop' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("RPOP key must be a string".to_string()),
                };

                Ok(Command::RPop { key })
            }
            "LRANGE" => {
                // LRANGE key start stop
                if array.len() != 4 {
                    return Err("ERR wrong number of arguments for 'lrange' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("LRANGE key must be a string".to_string()),
                };

                let start = match &array[2] {
                    Frame::Bulk(data) => {
                        let s = std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in start index")?;
                        s.parse::<isize>()
                            .map_err(|_| "ERR value is not an integer or out of range")?
                    }
                    Frame::Simple(s) => s
                        .parse::<isize>()
                        .map_err(|_| "ERR value is not an integer or out of range")?,
                    _ => return Err("ERR value is not an integer or out of range".to_string()),
                };

                let stop = match &array[3] {
                    Frame::Bulk(data) => {
                        let s = std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in stop index")?;
                        s.parse::<isize>()
                            .map_err(|_| "ERR value is not an integer or out of range")?
                    }
                    Frame::Simple(s) => s
                        .parse::<isize>()
                        .map_err(|_| "ERR value is not an integer or out of range")?,
                    _ => return Err("ERR value is not an integer or out of range".to_string()),
                };

                Ok(Command::LRange { key, start, stop })
            }
            "LLEN" => {
                // LLEN key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'llen' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("LLEN key must be a string".to_string()),
                };

                Ok(Command::LLen { key })
            }
            "SADD" => {
                // SADD key member [member ...]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'sadd' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SADD key must be a string".to_string()),
                };

                let mut members = Vec::new();
                for i in 2..array.len() {
                    let member = match &array[i] {
                        Frame::Bulk(data) => std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in member")?
                            .to_string(),
                        Frame::Simple(s) => s.clone(),
                        _ => return Err("SADD member must be a string".to_string()),
                    };
                    members.push(member);
                }

                Ok(Command::SAdd { key, members })
            }
            "SREM" => {
                // SREM key member [member ...]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'srem' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SREM key must be a string".to_string()),
                };

                let mut members = Vec::new();
                for i in 2..array.len() {
                    let member = match &array[i] {
                        Frame::Bulk(data) => std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in member")?
                            .to_string(),
                        Frame::Simple(s) => s.clone(),
                        _ => return Err("SREM member must be a string".to_string()),
                    };
                    members.push(member);
                }

                Ok(Command::SRem { key, members })
            }
            "SMEMBERS" => {
                // SMEMBERS key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'smembers' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SMEMBERS key must be a string".to_string()),
                };

                Ok(Command::SMembers { key })
            }
            "SISMEMBER" => {
                // SISMEMBER key member
                if array.len() != 3 {
                    return Err(
                        "ERR wrong number of arguments for 'sismember' command".to_string(),
                    );
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SISMEMBER key must be a string".to_string()),
                };

                let member = match &array[2] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in member")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SISMEMBER member must be a string".to_string()),
                };

                Ok(Command::SIsMember { key, member })
            }
            "SCARD" => {
                // SCARD key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'scard' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("SCARD key must be a string".to_string()),
                };

                Ok(Command::SCard { key })
            }
            "HSET" => {
                // HSET key field value
                if array.len() != 4 {
                    return Err("ERR wrong number of arguments for 'hset' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HSET key must be a string".to_string()),
                };

                let field = match &array[2] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in field")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HSET field must be a string".to_string()),
                };

                let value = match &array[3] {
                    Frame::Bulk(data) => data.clone(),
                    Frame::Simple(s) => Bytes::from(s.clone()),
                    _ => return Err("HSET value must be a string".to_string()),
                };

                Ok(Command::HSet { key, field, value })
            }
            "HGET" => {
                // HGET key field
                if array.len() != 3 {
                    return Err("ERR wrong number of arguments for 'hget' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HGET key must be a string".to_string()),
                };

                let field = match &array[2] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in field")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HGET field must be a string".to_string()),
                };

                Ok(Command::HGet { key, field })
            }
            "HGETALL" => {
                // HGETALL key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'hgetall' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HGETALL key must be a string".to_string()),
                };

                Ok(Command::HGetAll { key })
            }
            "HDEL" => {
                // HDEL key field [field ...]
                if array.len() < 3 {
                    return Err("ERR wrong number of arguments for 'hdel' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HDEL key must be a string".to_string()),
                };

                let mut fields = Vec::new();
                for i in 2..array.len() {
                    let field = match &array[i] {
                        Frame::Bulk(data) => std::str::from_utf8(data)
                            .map_err(|_| "invalid UTF-8 in field")?
                            .to_string(),
                        Frame::Simple(s) => s.clone(),
                        _ => return Err("HDEL field must be a string".to_string()),
                    };
                    fields.push(field);
                }

                Ok(Command::HDel { key, fields })
            }
            "HEXISTS" => {
                // HEXISTS key field
                if array.len() != 3 {
                    return Err("ERR wrong number of arguments for 'hexists' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HEXISTS key must be a string".to_string()),
                };

                let field = match &array[2] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in field")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HEXISTS field must be a string".to_string()),
                };

                Ok(Command::HExists { key, field })
            }
            "HLEN" => {
                // HLEN key
                if array.len() != 2 {
                    return Err("ERR wrong number of arguments for 'hlen' command".to_string());
                }

                let key = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in key")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("HLEN key must be a string".to_string()),
                };

                Ok(Command::HLen { key })
            }
            "PUBLISH" => {
                // PUBLISH channel message
                if array.len() != 3 {
                    return Err("ERR wrong number of arguments for 'publish' command".to_string());
                }

                let channel = match &array[1] {
                    Frame::Bulk(data) => std::str::from_utf8(data)
                        .map_err(|_| "invalid UTF-8 in channel")?
                        .to_string(),
                    Frame::Simple(s) => s.clone(),
                    _ => return Err("PUBLISH channel must be a string".to_string()),
                };

                let message = match &array[2] {
                    Frame::Bulk(data) => data.clone(),
                    Frame::Simple(s) => Bytes::from(s.clone()),
                    _ => return Err("PUBLISH message must be a string".to_string()),
                };

                Ok(Command::Publish { channel, message })
            }
            _ => Ok(Command::Unknown(cmd_name)),
        }
    }

    /// Execute the command and write the response to the connection
    pub async fn execute(&self, db: &Db, dst: &mut Connection, pubsub: &PubSub) -> Result<(), io::Error> {
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
                db.write_string(key.clone(), value.clone(), *expires_at);

                // Return OK
                let response = Frame::Simple("OK".to_string());
                dst.write_frame(&response).await?;
            }
            Command::Get { key } => {
                // Read from database
                let response = if let Some(value) = db.read_string(key) {
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
            Command::Del { keys } => {
                // Delete keys and return count of deleted keys
                let mut count = 0;
                for key in keys {
                    if db.delete(key) {
                        count += 1;
                    }
                }
                let response = Frame::Integer(count);
                dst.write_frame(&response).await?;
            }
            Command::Exists { key } => {
                // Check if key exists
                let exists = db.exists(key);
                let response = Frame::Integer(if exists { 1 } else { 0 });
                dst.write_frame(&response).await?;
            }
            Command::Type { key } => {
                // Get the type of a value
                let type_name = db.get_type(key).unwrap_or("none");
                let response = Frame::Simple(type_name.to_string());
                dst.write_frame(&response).await?;
            }
            Command::DbSize => {
                // Get the number of keys in the database
                let size = db.dbsize();
                let response = Frame::Integer(size as i64);
                dst.write_frame(&response).await?;
            }
            Command::FlushDb => {
                // Clear all keys from the database
                db.flushdb();
                let response = Frame::Simple("OK".to_string());
                dst.write_frame(&response).await?;
            }
            Command::Keys { pattern } => {
                // Get all keys matching a pattern
                let keys = db.keys(pattern);
                let response = Frame::Array(
                    keys.into_iter()
                        .map(|k| Frame::Bulk(Bytes::from(k)))
                        .collect(),
                );
                dst.write_frame(&response).await?;
            }
            Command::LPush { key, values } => {
                // Push values to the left of a list
                let len = db.lpush(key.clone(), values.clone());
                let response = Frame::Integer(len as i64);
                dst.write_frame(&response).await?;
            }
            Command::RPush { key, values } => {
                // Push values to the right of a list
                let len = db.rpush(key.clone(), values.clone());
                let response = Frame::Integer(len as i64);
                dst.write_frame(&response).await?;
            }
            Command::LPop { key } => {
                // Pop a value from the left of a list
                let response = if let Some(value) = db.lpop(key) {
                    Frame::Bulk(value)
                } else {
                    Frame::Null
                };
                dst.write_frame(&response).await?;
            }
            Command::RPop { key } => {
                // Pop a value from the right of a list
                let response = if let Some(value) = db.rpop(key) {
                    Frame::Bulk(value)
                } else {
                    Frame::Null
                };
                dst.write_frame(&response).await?;
            }
            Command::LRange { key, start, stop } => {
                // Get a range of elements from a list
                let response = if let Some(values) = db.lrange(key, *start, *stop) {
                    Frame::Array(values.into_iter().map(Frame::Bulk).collect())
                } else {
                    Frame::Array(Vec::new())
                };
                dst.write_frame(&response).await?;
            }
            Command::LLen { key } => {
                // Get the length of a list
                let len = db.llen(key).unwrap_or(0);
                let response = Frame::Integer(len as i64);
                dst.write_frame(&response).await?;
            }
            Command::SAdd { key, members } => {
                // Add members to a set
                let added = db.sadd(key.clone(), members.clone());
                let response = Frame::Integer(added as i64);
                dst.write_frame(&response).await?;
            }
            Command::SRem { key, members } => {
                // Remove members from a set
                let removed = db.srem(key, members.clone());
                let response = Frame::Integer(removed as i64);
                dst.write_frame(&response).await?;
            }
            Command::SMembers { key } => {
                // Get all members of a set
                let response = if let Some(members) = db.smembers(key) {
                    Frame::Array(
                        members
                            .into_iter()
                            .map(|m| Frame::Bulk(Bytes::from(m)))
                            .collect(),
                    )
                } else {
                    Frame::Array(Vec::new())
                };
                dst.write_frame(&response).await?;
            }
            Command::SIsMember { key, member } => {
                // Check if a member exists in a set
                let exists = db.sismember(key, member);
                let response = Frame::Integer(if exists { 1 } else { 0 });
                dst.write_frame(&response).await?;
            }
            Command::SCard { key } => {
                // Get the cardinality of a set
                let card = db.scard(key);
                let response = Frame::Integer(card as i64);
                dst.write_frame(&response).await?;
            }
            Command::HSet { key, field, value } => {
                // Set a field in a hash
                let is_new = db.hset(key.clone(), field.clone(), value.clone());
                let response = Frame::Integer(if is_new { 1 } else { 0 });
                dst.write_frame(&response).await?;
            }
            Command::HGet { key, field } => {
                // Get a field from a hash
                let response = if let Some(value) = db.hget(key, field) {
                    Frame::Bulk(value)
                } else {
                    Frame::Null
                };
                dst.write_frame(&response).await?;
            }
            Command::HGetAll { key } => {
                // Get all fields and values from a hash
                let response = if let Some(pairs) = db.hgetall(key) {
                    let mut result = Vec::new();
                    for (field, value) in pairs {
                        result.push(Frame::Bulk(Bytes::from(field)));
                        result.push(Frame::Bulk(value));
                    }
                    Frame::Array(result)
                } else {
                    Frame::Array(Vec::new())
                };
                dst.write_frame(&response).await?;
            }
            Command::HDel { key, fields } => {
                // Delete fields from a hash
                let deleted = db.hdel(key, fields.clone());
                let response = Frame::Integer(deleted as i64);
                dst.write_frame(&response).await?;
            }
            Command::HExists { key, field } => {
                // Check if a field exists in a hash
                let exists = db.hexists(key, field);
                let response = Frame::Integer(if exists { 1 } else { 0 });
                dst.write_frame(&response).await?;
            }
            Command::HLen { key } => {
                // Get the number of fields in a hash
                let len = db.hlen(key);
                let response = Frame::Integer(len as i64);
                dst.write_frame(&response).await?;
            }
            Command::Publish { channel, message } => {
                // Publish a message to a channel
                let num_receivers = pubsub.publish(channel, message.clone());
                let response = Frame::Integer(num_receivers as i64);
                dst.write_frame(&response).await?;
            }
            Command::Unknown(cmd) => {
                let error = Frame::error(format!("ERR unknown command '{}'", cmd));
                dst.write_frame(&error).await?;
            }
        }
        Ok(())
    }

    /// Check if this command modifies data (for AOF logging)
    pub fn is_write_command(&self) -> bool {
        matches!(
            self,
            Command::Set { .. }
                | Command::Del { .. }
                | Command::FlushDb
                | Command::LPush { .. }
                | Command::RPush { .. }
                | Command::LPop { .. }
                | Command::RPop { .. }
                | Command::SAdd { .. }
                | Command::SRem { .. }
                | Command::HSet { .. }
                | Command::HDel { .. }
        )
    }

    /// Replay a command without sending a response (for AOF restore)
    pub fn replay(&self, db: &Db) -> Result<(), String> {
        match self {
            Command::Set {
                key,
                value,
                expires_at,
            } => {
                db.write_string(key.clone(), value.clone(), *expires_at);
                Ok(())
            }
            Command::Del { keys } => {
                for key in keys {
                    db.delete(key);
                }
                Ok(())
            }
            Command::FlushDb => {
                db.flushdb();
                Ok(())
            }
            Command::LPush { key, values } => {
                db.lpush(key.clone(), values.clone());
                Ok(())
            }
            Command::RPush { key, values } => {
                db.rpush(key.clone(), values.clone());
                Ok(())
            }
            Command::LPop { key } => {
                db.lpop(key);
                Ok(())
            }
            Command::RPop { key } => {
                db.rpop(key);
                Ok(())
            }
            Command::SAdd { key, members } => {
                db.sadd(key.clone(), members.clone());
                Ok(())
            }
            Command::SRem { key, members } => {
                db.srem(key, members.clone());
                Ok(())
            }
            Command::HSet { key, field, value } => {
                db.hset(key.clone(), field.clone(), value.clone());
                Ok(())
            }
            Command::HDel { key, fields } => {
                db.hdel(key, fields.clone());
                Ok(())
            }
            _ => Ok(()), // Read-only commands don't need replay
        }
    }
}
