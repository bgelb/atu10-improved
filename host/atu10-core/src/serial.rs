use crate::{Error, Result};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::time::Duration;

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

pub struct SystemSerialTunnel {
    port: Box<dyn SerialPort>,
}

impl SystemSerialTunnel {
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(2000))
            .open()
            .map_err(|err| Error::Serial(format!("failed to open serial port {path}: {err}")))?;
        Ok(Self { port })
    }
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

impl SerialTunnel for SystemSerialTunnel {
    fn read_line(&mut self) -> Result<String> {
        let mut line = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            self.port
                .read_exact(&mut byte)
                .map_err(|err| Error::Serial(format!("serial read failed: {err}")))?;
            if byte[0] == b'\n' {
                break;
            }
            if byte[0] != b'\r' {
                line.push(byte[0]);
            }
            if line.len() > 256 {
                return Err(Error::Serial("serial line exceeded 256 bytes".to_string()));
            }
        }

        String::from_utf8(line)
            .map_err(|err| Error::Serial(format!("serial line was not UTF-8: {err}")))
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.port
            .write_all(bytes)
            .map_err(|err| Error::Serial(format!("serial write failed: {err}")))
    }

    fn read_exact(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0u8; len];
        self.port
            .read_exact(&mut bytes)
            .map_err(|err| Error::Serial(format!("serial read failed: {err}")))?;
        Ok(bytes)
    }
}

pub fn serial_smoke<T: SerialTunnel + ?Sized>(serial: &mut T) -> Result<()> {
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
