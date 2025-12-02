use bytes::Bytes;
use std::fmt;

/// Represents a Redis RESP (REdis Serialization Protocol) frame.
/// 
/// RESP defines 6 data types:
/// - Simple Strings: +OK\r\n
/// - Errors: -Error message\r\n
/// - Integers: :1000\r\n
/// - Bulk Strings: $5\r\nhello\r\n
/// - Arrays: *2\r\n$3\r\nGET\r\n$3\r\nkey\r\n
/// - Null: $-1\r\n
#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    /// Simple string: +OK\r\n
    Simple(String),
    
    /// Error message: -Error message\r\n
    Error(String),
    
    /// Integer value: :1000\r\n
    Integer(i64),
    
    /// Bulk string: $5\r\nhello\r\n
    /// Uses Bytes for zero-copy operations
    Bulk(Bytes),
    
    /// Array of frames: *2\r\n$3\r\nGET\r\n$3\r\nkey\r\n
    Array(Vec<Frame>),
    
    /// Null bulk string: $-1\r\n
    Null,
}

impl Frame {
    /// Create a Simple String frame
    pub fn simple(s: impl Into<String>) -> Frame {
        Frame::Simple(s.into())
    }
    
    /// Create an Error frame
    pub fn error(msg: impl Into<String>) -> Frame {
        Frame::Error(msg.into())
    }
    
    /// Create an Integer frame
    pub fn integer(n: i64) -> Frame {
        Frame::Integer(n)
    }
    
    /// Create a Bulk String frame
    pub fn bulk(data: Bytes) -> Frame {
        Frame::Bulk(data)
    }
    
    /// Create an Array frame
    pub fn array(items: Vec<Frame>) -> Frame {
        Frame::Array(items)
    }
    
    /// Create a Null frame
    pub fn null() -> Frame {
        Frame::Null
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Frame::Simple(s) => write!(f, "Simple({})", s),
            Frame::Error(e) => write!(f, "Error({})", e),
            Frame::Integer(i) => write!(f, "Integer({})", i),
            Frame::Bulk(b) => {
                if let Ok(s) = std::str::from_utf8(b) {
                    write!(f, "Bulk({})", s)
                } else {
                    write!(f, "Bulk({} bytes)", b.len())
                }
            }
            Frame::Array(arr) => {
                write!(f, "Array[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Frame::Null => write!(f, "Null"),
        }
    }
}
