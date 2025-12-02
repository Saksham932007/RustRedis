use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use crate::frame::{Frame, Error as FrameError};
use std::io;
use std::pin::Pin;
use std::future::Future;

/// Connection wrapper around a TcpStream that handles buffered reading/writing
/// and frame parsing/serialization
pub struct Connection {
    /// The underlying TCP stream wrapped in a buffered writer
    stream: BufWriter<TcpStream>,
    
    /// Read buffer for incoming data
    buffer: BytesMut,
}

impl Connection {
    /// Create a new Connection from a TcpStream
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4096),
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
            Err(FrameError::Invalid(msg)) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                msg,
            )),
            Err(FrameError::Io(e)) => Err(e),
        }
    }
    
    /// Write a frame to the connection
    pub async fn write_frame(&mut self, frame: &Frame) -> Result<(), io::Error> {
        // Serialize the frame to the writer
        self.write_value(frame).await?;
        
        // Flush the buffer to ensure data is sent
        self.stream.flush().await?;
        
        Ok(())
    }
    
    /// Serialize a frame value to the writer
    fn write_value<'a>(&'a mut self, frame: &'a Frame) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + 'a>> {
        Box::pin(async move {
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
                    self.stream.write_all(data.len().to_string().as_bytes()).await?;
                    self.stream.write_all(b"\r\n").await?;
                    self.stream.write_all(data).await?;
                    self.stream.write_all(b"\r\n").await?;
                }
                Frame::Array(frames) => {
                    self.stream.write_u8(b'*').await?;
                    self.stream.write_all(frames.len().to_string().as_bytes()).await?;
                    self.stream.write_all(b"\r\n").await?;
                    
                    for frame in frames {
                        self.write_value(frame).await?;
                    }
                }
            }
            
            Ok(())
        })
    }
}
