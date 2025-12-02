use bytes::{Bytes, BytesMut, Buf};
use std::fmt;
use std::io::{self, Cursor};

/// Error type for frame parsing
#[derive(Debug)]
pub enum Error {
    /// Not enough data to parse a complete frame
    Incomplete,
    
    /// Invalid frame format
    Invalid(String),
    
    /// IO error
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Incomplete => write!(f, "incomplete frame"),
            Error::Invalid(msg) => write!(f, "invalid frame: {}", msg),
            Error::Io(err) => write!(f, "io error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

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
    
    /// Parse a frame from the buffer
    /// 
    /// Returns `Ok(Some(frame))` if a complete frame was parsed
    /// Returns `Ok(None)` if there is not enough data yet (incomplete)
    /// Returns `Err` if the data is malformed
    pub fn parse(buf: &mut BytesMut) -> Result<Option<Frame>, Error> {
        // Create a cursor to track position without consuming
        let mut cursor = Cursor::new(&buf[..]);
        
        // Check if we have a complete frame
        match check_complete(&mut cursor) {
            Ok(_) => {
                // We have a complete frame, now parse it
                let len = cursor.position() as usize;
                
                // Reset cursor for actual parsing
                cursor.set_position(0);
                
                // Parse the frame
                let frame = parse_frame(&mut cursor)?;
                
                // Advance the buffer past the parsed frame
                buf.advance(len);
                
                Ok(Some(frame))
            }
            Err(Error::Incomplete) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Check if a complete frame is available in the buffer
fn check_complete(cursor: &mut Cursor<&[u8]>) -> Result<(), Error> {
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    }
    
    match get_u8(cursor)? {
        b'+' => read_until_crlf(cursor),  // Simple String
        b'-' => read_until_crlf(cursor),  // Error
        b':' => read_until_crlf(cursor),  // Integer
        b'$' => {
            // Bulk String
            let len = read_decimal(cursor)?;
            if len == -1 {
                // Null bulk string
                Ok(())
            } else {
                // Skip len bytes + \r\n
                skip(cursor, len as usize + 2)
            }
        }
        b'*' => {
            // Array
            let count = read_decimal(cursor)?;
            if count == -1 {
                // Null array (not standard but handle it)
                Ok(())
            } else {
                // Recursively check each element
                for _ in 0..count {
                    check_complete(cursor)?;
                }
                Ok(())
            }
        }
        actual => Err(Error::Invalid(format!("invalid frame type byte: {}", actual))),
    }
}

/// Parse a complete frame from the cursor
fn parse_frame(cursor: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
    match get_u8(cursor)? {
        b'+' => {
            let line = read_line(cursor)?;
            let string = String::from_utf8(line.to_vec())
                .map_err(|_| Error::Invalid("invalid UTF-8 in simple string".to_string()))?;
            Ok(Frame::Simple(string))
        }
        b'-' => {
            let line = read_line(cursor)?;
            let string = String::from_utf8(line.to_vec())
                .map_err(|_| Error::Invalid("invalid UTF-8 in error".to_string()))?;
            Ok(Frame::Error(string))
        }
        b':' => {
            let num = read_decimal(cursor)?;
            Ok(Frame::Integer(num))
        }
        b'$' => {
            let len = read_decimal(cursor)?;
            if len == -1 {
                Ok(Frame::Null)
            } else {
                let data = read_n_bytes(cursor, len as usize)?;
                skip(cursor, 2)?; // Skip \r\n
                Ok(Frame::Bulk(Bytes::copy_from_slice(data)))
            }
        }
        b'*' => {
            let count = read_decimal(cursor)?;
            if count == -1 {
                Ok(Frame::Null)
            } else {
                let mut frames = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    frames.push(parse_frame(cursor)?);
                }
                Ok(Frame::Array(frames))
            }
        }
        _ => Err(Error::Invalid("invalid frame type".to_string())),
    }
}

/// Read a single byte from the cursor
fn get_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    }
    Ok(cursor.get_u8())
}

/// Read until \r\n and verify it exists
fn read_until_crlf(cursor: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let start = cursor.position() as usize;
    let slice = &cursor.get_ref()[start..];
    
    for i in 0..slice.len() {
        if i + 1 < slice.len() && slice[i] == b'\r' && slice[i + 1] == b'\n' {
            cursor.set_position((start + i + 2) as u64);
            return Ok(());
        }
    }
    
    Err(Error::Incomplete)
}

/// Read a line (until \r\n) and return it without the \r\n
fn read_line<'a>(cursor: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    let start = cursor.position() as usize;
    let slice = &cursor.get_ref()[start..];
    
    for i in 0..slice.len() {
        if i + 1 < slice.len() && slice[i] == b'\r' && slice[i + 1] == b'\n' {
            cursor.set_position((start + i + 2) as u64);
            return Ok(&slice[..i]);
        }
    }
    
    Err(Error::Incomplete)
}

/// Read a decimal integer followed by \r\n
fn read_decimal(cursor: &mut Cursor<&[u8]>) -> Result<i64, Error> {
    let line = read_line(cursor)?;
    let string = std::str::from_utf8(line)
        .map_err(|_| Error::Invalid("invalid UTF-8 in decimal".to_string()))?;
    
    string.parse::<i64>()
        .map_err(|_| Error::Invalid(format!("invalid decimal: {}", string)))
}

/// Read exactly n bytes
fn read_n_bytes<'a>(cursor: &mut Cursor<&'a [u8]>, n: usize) -> Result<&'a [u8], Error> {
    let start = cursor.position() as usize;
    let end = start + n;
    
    if end > cursor.get_ref().len() {
        return Err(Error::Incomplete);
    }
    
    cursor.set_position(end as u64);
    Ok(&cursor.get_ref()[start..end])
}

/// Skip n bytes
fn skip(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<(), Error> {
    let new_pos = cursor.position() as usize + n;
    
    if new_pos > cursor.get_ref().len() {
        return Err(Error::Incomplete);
    }
    
    cursor.set_position(new_pos as u64);
    Ok(())
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
