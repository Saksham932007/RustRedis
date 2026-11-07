#![no_main]

use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;
use rust_redis::{cmd::Command, frame::Frame};

// Feed arbitrary bytes through the same path a client connection takes:
// RESP framing first, then command parsing. Neither may panic or overflow
// the stack, whatever the input.
fuzz_target!(|data: &[u8]| {
    let mut buf = BytesMut::from(data);
    while let Ok(Some(frame)) = Frame::parse(&mut buf) {
        let _ = Command::from_frame(&frame);
    }
});
