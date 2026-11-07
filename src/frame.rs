use bytes::{Buf, Bytes, BytesMut};
use std::fmt;
use std::io::{self, Cursor};

/// Maximum nesting depth of arrays, matching the limit Redis applies.
const MAX_DEPTH: usize = 128;

/// Maximum bulk string length (512 MB, Redis' `proto-max-bulk-len` default).
const MAX_BULK_LEN: usize = 512 * 1024 * 1024;

/// Maximum number of elements in an array (Redis' multibulk limit).
const MAX_ARRAY_LEN: usize = 1024 * 1024;

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
    /// The payload is copied out of the read buffer when the frame is parsed;
    /// `Bytes` makes later clones cheap.
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
        match check_complete(&mut cursor, 0) {
            Ok(_) => {
                // We have a complete frame, now parse it
                let len = cursor.position() as usize;

                // Reset cursor for actual parsing
                cursor.set_position(0);

                // Parse the frame
                let frame = parse_frame(&mut cursor, 0)?;

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
fn check_complete(cursor: &mut Cursor<&[u8]>, depth: usize) -> Result<(), Error> {
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    }

    match get_u8(cursor)? {
        b'+' => read_until_crlf(cursor), // Simple String
        b'-' => read_until_crlf(cursor), // Error
        b':' => read_until_crlf(cursor), // Integer
        b'$' => {
            // Bulk String
            match read_length(cursor, "bulk", MAX_BULK_LEN)? {
                // Null bulk string
                None => Ok(()),
                // Skip len bytes + \r\n
                Some(len) => skip(cursor, len.saturating_add(2)),
            }
        }
        b'*' => {
            // Array
            match read_length(cursor, "array", MAX_ARRAY_LEN)? {
                // Null array (not standard but handle it)
                None => Ok(()),
                // Recursively check each element
                Some(_) if depth >= MAX_DEPTH => Err(too_deep()),
                Some(count) => {
                    for _ in 0..count {
                        check_complete(cursor, depth + 1)?;
                    }
                    Ok(())
                }
            }
        }
        actual => Err(Error::Invalid(format!(
            "invalid frame type byte: {}",
            actual
        ))),
    }
}

fn too_deep() -> Error {
    Error::Invalid(format!("array nesting exceeds {} levels", MAX_DEPTH))
}

/// Parse a complete frame from the cursor
fn parse_frame(cursor: &mut Cursor<&[u8]>, depth: usize) -> Result<Frame, Error> {
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
        b'$' => match read_length(cursor, "bulk", MAX_BULK_LEN)? {
            None => Ok(Frame::Null),
            Some(len) => {
                let data = read_n_bytes(cursor, len)?;
                skip(cursor, 2)?; // Skip \r\n
                Ok(Frame::Bulk(Bytes::copy_from_slice(data)))
            }
        },
        b'*' => match read_length(cursor, "array", MAX_ARRAY_LEN)? {
            None => Ok(Frame::Null),
            Some(_) if depth >= MAX_DEPTH => Err(too_deep()),
            Some(count) => {
                let mut frames = Vec::with_capacity(count);
                for _ in 0..count {
                    frames.push(parse_frame(cursor, depth + 1)?);
                }
                Ok(Frame::Array(frames))
            }
        },
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

    string
        .parse::<i64>()
        .map_err(|_| Error::Invalid(format!("invalid decimal: {}", string)))
}

/// Read a length prefix. `-1` means null and yields `None`; any other
/// negative value, or one above `max`, is a protocol error.
fn read_length(cursor: &mut Cursor<&[u8]>, what: &str, max: usize) -> Result<Option<usize>, Error> {
    match read_decimal(cursor)? {
        -1 => Ok(None),
        n => usize::try_from(n)
            .ok()
            .filter(|&len| len <= max)
            .map(Some)
            .ok_or_else(|| Error::Invalid(format!("invalid {} length: {}", what, n))),
    }
}

/// Read exactly n bytes
fn read_n_bytes<'a>(cursor: &mut Cursor<&'a [u8]>, n: usize) -> Result<&'a [u8], Error> {
    let start = cursor.position() as usize;
    let end = start
        .checked_add(n)
        .ok_or_else(|| Error::Invalid("length overflow".to_string()))?;

    if end > cursor.get_ref().len() {
        return Err(Error::Incomplete);
    }

    cursor.set_position(end as u64);
    Ok(&cursor.get_ref()[start..end])
}

/// Skip n bytes
fn skip(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<(), Error> {
    let new_pos = (cursor.position() as usize)
        .checked_add(n)
        .ok_or_else(|| Error::Invalid("length overflow".to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Result<Option<Frame>, Error> {
        let mut buf = BytesMut::from(input);
        Frame::parse(&mut buf)
    }

    #[test]
    fn parses_simple_string() {
        assert_eq!(parse(b"+OK\r\n").unwrap(), Some(Frame::simple("OK")));
    }

    #[test]
    fn parses_error() {
        assert_eq!(parse(b"-ERR bad\r\n").unwrap(), Some(Frame::error("ERR bad")));
    }

    #[test]
    fn parses_integer() {
        assert_eq!(parse(b":-42\r\n").unwrap(), Some(Frame::integer(-42)));
    }

    #[test]
    fn parses_bulk_string() {
        assert_eq!(
            parse(b"$5\r\nhello\r\n").unwrap(),
            Some(Frame::bulk(Bytes::from_static(b"hello")))
        );
    }

    #[test]
    fn parses_empty_bulk_string() {
        assert_eq!(
            parse(b"$0\r\n\r\n").unwrap(),
            Some(Frame::bulk(Bytes::new()))
        );
    }

    #[test]
    fn parses_null_bulk_string() {
        assert_eq!(parse(b"$-1\r\n").unwrap(), Some(Frame::Null));
    }

    #[test]
    fn parses_array_and_nested_array() {
        let frame = parse(b"*2\r\n$3\r\nGET\r\n*1\r\n:1\r\n").unwrap().unwrap();
        assert_eq!(
            frame,
            Frame::array(vec![
                Frame::bulk(Bytes::from_static(b"GET")),
                Frame::array(vec![Frame::integer(1)]),
            ])
        );
    }

    #[test]
    fn bulk_may_contain_crlf_and_binary() {
        assert_eq!(
            parse(b"$4\r\n\r\n\xff\x00\r\n").unwrap(),
            Some(Frame::bulk(Bytes::from_static(b"\r\n\xff\x00")))
        );
    }

    #[test]
    fn incomplete_inputs_return_none() {
        for input in [
            &b""[..],
            b"+OK",
            b"+OK\r",
            b"$5\r\nhel",
            b"$5\r\nhello",
            b"*2\r\n$3\r\nGET\r\n",
            b"$5",
        ] {
            assert!(parse(input).unwrap().is_none(), "input {:?}", input);
        }
    }

    #[test]
    fn incomplete_parse_does_not_consume_buffer() {
        let mut buf = BytesMut::from(&b"$5\r\nhel"[..]);
        assert!(Frame::parse(&mut buf).unwrap().is_none());
        assert_eq!(&buf[..], b"$5\r\nhel");
    }

    #[test]
    fn parse_consumes_exactly_one_frame() {
        let mut buf = BytesMut::from(&b"+OK\r\n:1\r\n"[..]);
        assert_eq!(Frame::parse(&mut buf).unwrap(), Some(Frame::simple("OK")));
        assert_eq!(&buf[..], b":1\r\n");
    }

    #[test]
    fn invalid_type_byte_is_rejected() {
        assert!(matches!(parse(b"?foo\r\n"), Err(Error::Invalid(_))));
    }

    #[test]
    fn negative_array_count_below_minus_one_is_rejected() {
        assert!(matches!(parse(b"*-5\r\n"), Err(Error::Invalid(_))));
        assert!(matches!(parse(b"*-2\r\n"), Err(Error::Invalid(_))));
    }

    #[test]
    fn negative_bulk_length_below_minus_one_is_rejected() {
        assert!(matches!(parse(b"$-5\r\n"), Err(Error::Invalid(_))));
        assert!(matches!(
            parse(b"$-9223372036854775808\r\n"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn length_near_i64_max_is_rejected_not_overflowed() {
        assert!(matches!(
            parse(b"$9223372036854775807\r\n"),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            parse(b"*9223372036854775807\r\n"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn bulk_length_limit_is_enforced() {
        let at_limit = format!("${}\r\n", MAX_BULK_LEN);
        let over_limit = format!("${}\r\n", MAX_BULK_LEN + 1);
        assert!(parse(at_limit.as_bytes()).unwrap().is_none());
        assert!(matches!(parse(over_limit.as_bytes()), Err(Error::Invalid(_))));
    }

    #[test]
    fn array_length_limit_is_enforced() {
        let over_limit = format!("*{}\r\n", MAX_ARRAY_LEN + 1);
        assert!(matches!(parse(over_limit.as_bytes()), Err(Error::Invalid(_))));
    }

    fn nested(levels: usize) -> Vec<u8> {
        let mut input = b"*1\r\n".repeat(levels);
        input.extend_from_slice(b":1\r\n");
        input
    }

    #[test]
    fn nesting_at_the_limit_is_accepted() {
        assert!(parse(&nested(MAX_DEPTH)).unwrap().is_some());
    }

    #[test]
    fn nesting_beyond_the_limit_is_rejected() {
        assert!(matches!(
            parse(&nested(MAX_DEPTH + 1)),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn extreme_nesting_does_not_overflow_the_stack() {
        // 400 KB of input; used to recurse 100k frames deep.
        assert!(matches!(parse(&nested(100_000)), Err(Error::Invalid(_))));
    }

    #[test]
    fn null_array_is_accepted() {
        assert_eq!(parse(b"*-1\r\n").unwrap(), Some(Frame::Null));
    }

    #[test]
    fn non_numeric_length_is_rejected() {
        assert!(matches!(parse(b"$abc\r\n"), Err(Error::Invalid(_))));
        assert!(matches!(parse(b":1x\r\n"), Err(Error::Invalid(_))));
    }
}
