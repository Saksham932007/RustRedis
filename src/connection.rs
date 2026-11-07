use crate::frame::{Error as FrameError, Frame, MAX_BULK_LEN};
use bytes::BytesMut;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

/// Default ceiling for unparsed bytes buffered per connection: the largest
/// legal bulk string plus room for its header and framing.
pub const DEFAULT_MAX_BUFFER: usize = MAX_BULK_LEN + 1024 * 1024;

/// Connection wrapper around a TcpStream that handles buffered reading/writing
/// and frame parsing/serialization
pub struct Connection {
    /// The underlying TCP stream wrapped in a buffered writer
    stream: BufWriter<TcpStream>,

    /// Read buffer for incoming data
    buffer: BytesMut,

    /// Maximum number of unparsed bytes to buffer before giving up on the peer
    max_buffer: usize,
}

impl Connection {
    /// Create a new Connection from a TcpStream
    pub fn new(socket: TcpStream) -> Connection {
        Connection::with_max_buffer(socket, DEFAULT_MAX_BUFFER)
    }

    /// Create a Connection that refuses to buffer more than `max_buffer`
    /// unparsed bytes from the peer
    pub fn with_max_buffer(socket: TcpStream, max_buffer: usize) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4096),
            max_buffer,
        }
    }

    /// Read a frame from the connection
    ///
    /// Returns `Ok(Some(frame))` if a frame was read
    /// Returns `Ok(None)` if the connection was closed
    /// Returns `Err` on IO or parsing errors
    pub async fn read_frame(&mut self) -> Result<Option<Frame>, io::Error> {
        loop {
            // Try to parse a frame from the buffer
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // A peer that never completes a frame must not grow the buffer forever
            if self.buffer.len() > self.max_buffer {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "request exceeds maximum buffer size",
                ));
            }

            // About to wait for the client: send everything queued so far. When
            // the client pipelines several commands this flushes once per batch
            // rather than once per response.
            self.flush().await?;

            // Not enough data, read more from the socket
            let bytes_read = self.stream.read_buf(&mut self.buffer).await?;

            // If 0 bytes read, the connection is closed
            if bytes_read == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "connection reset by peer",
                    ));
                }
            }
        }
    }

    /// Try to parse a frame from the buffer
    fn parse_frame(&mut self) -> Result<Option<Frame>, io::Error> {
        match Frame::parse(&mut self.buffer) {
            Ok(frame) => Ok(frame),
            Err(FrameError::Incomplete) => Ok(None),
            Err(FrameError::Invalid(msg)) => Err(io::Error::new(io::ErrorKind::InvalidData, msg)),
            Err(FrameError::Io(e)) => Err(e),
        }
    }

    /// Queue a frame for sending
    ///
    /// The frame is buffered; it is sent when the buffer fills, when
    /// `read_frame` is about to wait for more input, or on an explicit
    /// `flush`.
    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        self.write_value(frame).await
    }

    /// Send all buffered frames to the peer
    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.stream.flush().await
    }

    /// Serialize a frame value to the writer
    async fn write_value(&mut self, frame: &Frame) -> Result<(), io::Error> {
        match frame {
            Frame::Simple(s) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(s.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(e) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(e.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(n) => {
                self.stream.write_u8(b':').await?;
                self.stream.write_all(n.to_string().as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Bulk(data) => {
                self.stream.write_u8(b'$').await?;
                self.stream
                    .write_all(data.len().to_string().as_bytes())
                    .await?;
                self.stream.write_all(b"\r\n").await?;
                self.stream.write_all(data).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Array(frames) => {
                self.stream.write_u8(b'*').await?;
                self.stream
                    .write_all(frames.len().to_string().as_bytes())
                    .await?;
                self.stream.write_all(b"\r\n").await?;

                // Recursively write each frame in the array
                for frame in frames {
                    Box::pin(self.write_value(frame)).await?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// Returns a server-side Connection and the client socket talking to it
    async fn pair(max_buffer: usize) -> (Connection, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
        let (socket, _) = accepted.unwrap();
        (Connection::with_max_buffer(socket, max_buffer), client.unwrap())
    }

    #[tokio::test]
    async fn reads_a_frame() {
        let (mut conn, mut client) = pair(1024).await;
        client.write_all(b"+PONG\r\n").await.unwrap();
        assert_eq!(conn.read_frame().await.unwrap(), Some(Frame::simple("PONG")));
    }

    #[tokio::test]
    async fn write_frame_is_buffered_until_flush() {
        use tokio::io::AsyncReadExt;
        use tokio::time::{timeout, Duration};

        let (mut conn, mut client) = pair(1024).await;
        conn.write_frame(&Frame::simple("A")).await.unwrap();
        conn.write_frame(&Frame::simple("B")).await.unwrap();

        let mut buf = [0u8; 16];
        assert!(timeout(Duration::from_millis(100), client.read(&mut buf))
            .await
            .is_err());

        conn.flush().await.unwrap();
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"+A\r\n+B\r\n");
    }

    #[tokio::test]
    async fn read_frame_flushes_pending_responses_before_waiting() {
        use tokio::io::AsyncReadExt;

        let (mut conn, mut client) = pair(1024).await;
        conn.write_frame(&Frame::simple("OK")).await.unwrap();
        client.write_all(b"+X\r\n").await.unwrap();
        assert_eq!(conn.read_frame().await.unwrap(), Some(Frame::simple("X")));

        // The first read found a complete frame already buffered or read it
        // after flushing; either way a second read must flush the response.
        client.write_all(b"+Y\r\n").await.unwrap();
        assert_eq!(conn.read_frame().await.unwrap(), Some(Frame::simple("Y")));
        let mut buf = [0u8; 8];
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"+OK\r\n");
    }

    #[tokio::test]
    async fn input_without_a_terminator_hits_the_buffer_limit() {
        let (mut conn, mut client) = pair(1024).await;
        client.write_all(&[b'A'; 4096]).await.unwrap();
        let err = conn.read_frame().await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn invalid_frame_is_an_invalid_data_error() {
        let (mut conn, mut client) = pair(1024).await;
        client.write_all(b"*-5\r\n").await.unwrap();
        let err = conn.read_frame().await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
