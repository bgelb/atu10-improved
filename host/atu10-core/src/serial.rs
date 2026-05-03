use crate::{Error, Result};
use std::collections::VecDeque;

pub trait SerialTunnel {
    fn read_line(&mut self) -> Result<String>;
    fn write(&mut self, bytes: &[u8]) -> Result<()>;
    fn read_exact(&mut self, len: usize) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone)]
pub struct FakeSerialTunnel {
    lines: VecDeque<String>,
    rx: VecDeque<u8>,
}

impl Default for FakeSerialTunnel {
    fn default() -> Self {
        let mut lines = VecDeque::new();
        lines.push_back("ATU10-IMPROVED READY 115200".to_string());
        Self {
            lines,
            rx: VecDeque::new(),
        }
    }
}

impl SerialTunnel for FakeSerialTunnel {
    fn read_line(&mut self) -> Result<String> {
        self.lines
            .pop_front()
            .ok_or_else(|| Error::Serial("no line available".to_string()))
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        for byte in bytes {
            self.rx.push_back(*byte);
        }
        Ok(())
    }

    fn read_exact(&mut self, len: usize) -> Result<Vec<u8>> {
        if self.rx.len() < len {
            return Err(Error::Serial(format!(
                "wanted {len} bytes, only {} available",
                self.rx.len()
            )));
        }

        Ok((0..len).filter_map(|_| self.rx.pop_front()).collect())
    }
}

pub fn serial_smoke<T: SerialTunnel>(serial: &mut T) -> Result<()> {
    let hello = serial.read_line()?;
    if !hello.contains("ATU10-IMPROVED READY") {
        return Err(Error::Serial(format!("unexpected hello line: {hello}")));
    }

    let ping = b"ping\n";
    serial.write(ping)?;
    let echo = serial.read_exact(ping.len())?;
    if echo != ping {
        return Err(Error::Serial("echo mismatch".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_serial_supports_smoke_test() {
        let mut serial = FakeSerialTunnel::default();
        serial_smoke(&mut serial).unwrap();
    }

    #[test]
    fn bad_hello_fails_smoke_test() {
        let mut serial = FakeSerialTunnel {
            lines: VecDeque::from(["not ready".to_string()]),
            rx: VecDeque::new(),
        };
        assert!(serial_smoke(&mut serial).is_err());
    }
}
