use crate::frame::Frame;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;

/// AOF sync policy - determines when to sync writes to disk
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AofSyncPolicy {
    /// Sync after every write (safest, slowest)
    Always,
    /// Sync every second (balanced)
    EverySecond,
    /// Never sync explicitly, let OS decide (fastest, least safe)
    No,
}

/// AOF (Append-Only File) persistence layer
pub struct Aof {
    /// File handle for writing commands
    file: Arc<Mutex<File>>,
    /// Sync policy
    sync_policy: AofSyncPolicy,
}

impl Aof {
    /// Create a new AOF instance
    ///
    /// Opens (or creates) the AOF file at the given path
    pub fn new(path: impl AsRef<Path>, sync_policy: AofSyncPolicy) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Aof {
            file: Arc::new(Mutex::new(file)),
            sync_policy,
        })
    }

    /// Append a command to the AOF
    ///
    /// Serializes the frame and writes it to the file
    pub fn append(&self, frame: &Frame) -> io::Result<()> {
        let mut file = self.file.lock().unwrap();

        // Serialize the frame as RESP
        let serialized = Self::serialize_frame(frame);
        file.write_all(&serialized)?;

        // Sync based on policy
        if self.sync_policy == AofSyncPolicy::Always {
            file.sync_all()?;
        }

        Ok(())
    }

    /// Start background sync task for EverySecond policy
    pub fn start_background_sync(self: Arc<Self>) {
        if self.sync_policy != AofSyncPolicy::EverySecond {
            return;
        }

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Ok(file) = self.file.lock() {
                    let _ = file.sync_all();
                }
            }
        });
    }

    /// Load and replay all commands from the AOF file
    ///
    /// Returns a vector of frames that can be executed to restore state
    pub fn load(path: impl AsRef<Path>) -> io::Result<Vec<Frame>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut frames = Vec::new();
        let mut lines = reader.lines();

        while let Some(Ok(line)) = lines.next() {
            // Parse RESP frames
            if let Ok(frame) = Self::parse_line(&line, &mut lines) {
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    /// Serialize a frame to RESP format
    fn serialize_frame(frame: &Frame) -> Vec<u8> {
        let mut buf = Vec::new();
        Self::write_frame_recursive(frame, &mut buf);
        buf
    }

    /// Recursively write a frame to a buffer
    fn write_frame_recursive(frame: &Frame, buf: &mut Vec<u8>) {
        match frame {
            Frame::Simple(s) => {
                buf.extend_from_slice(b"+");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            Frame::Error(e) => {
                buf.extend_from_slice(b"-");
                buf.extend_from_slice(e.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            Frame::Integer(i) => {
                buf.extend_from_slice(b":");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            Frame::Bulk(data) => {
                buf.extend_from_slice(b"$");
                buf.extend_from_slice(data.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(data);
                buf.extend_from_slice(b"\r\n");
            }
            Frame::Null => {
                buf.extend_from_slice(b"$-1\r\n");
            }
            Frame::Array(arr) => {
                buf.extend_from_slice(b"*");
                buf.extend_from_slice(arr.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
                for item in arr {
                    Self::write_frame_recursive(item, buf);
                }
            }
        }
    }

    /// Parse a single line into a frame (simplified parser for AOF replay)
    fn parse_line(
        line: &str,
        lines: &mut impl Iterator<Item = io::Result<String>>,
    ) -> io::Result<Frame> {
        if line.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty line"));
        }

        let first_char = line.chars().next().unwrap();
        match first_char {
            '+' => Ok(Frame::Simple(line[1..].to_string())),
            '-' => Ok(Frame::Error(line[1..].to_string())),
            ':' => {
                let num = line[1..]
                    .parse::<i64>()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid integer"))?;
                Ok(Frame::Integer(num))
            }
            '$' => {
                let len = line[1..]
                    .parse::<isize>()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid length"))?;

                if len == -1 {
                    return Ok(Frame::Null);
                }

                if let Some(Ok(data_line)) = lines.next() {
                    Ok(Frame::Bulk(data_line.into_bytes().into()))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "missing bulk data",
                    ))
                }
            }
            '*' => {
                let count = line[1..]
                    .parse::<usize>()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid count"))?;

                let mut array = Vec::with_capacity(count);
                for _ in 0..count {
                    if let Some(Ok(next_line)) = lines.next() {
                        array.push(Self::parse_line(&next_line, lines)?);
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "incomplete array",
                        ));
                    }
                }
                Ok(Frame::Array(array))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown frame type",
            )),
        }
    }
}
