use std::io::{self, Write};

use serde::Serialize;

pub(crate) struct ByteCounter {
    remaining: usize,
    written: usize,
}

impl Write for ByteCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("output byte limit"));
        }
        self.remaining -= bytes.len();
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Count compact JSON bytes without allocating the serialized output.
pub(crate) fn json_size(value: &impl Serialize, limit: usize) -> Option<usize> {
    let mut counter = ByteCounter {
        remaining: limit,
        written: 0,
    };
    serde_json::to_writer(&mut counter, value).ok()?;
    Some(counter.written)
}
