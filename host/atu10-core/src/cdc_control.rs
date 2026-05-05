use crate::device::{ProbeInfo, ProgrammerDevice};
use crate::hid::verify_digest;
use crate::protocol::{
    decode_response, encode_request, Command, Request, Status, HID_PACKET_SIZE, MAX_PAYLOAD_SIZE,
};
use crate::serial::SerialTunnel;
use crate::{Error, Result};
use serialport::SerialPort;
use std::fmt::Write as _;
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;

pub struct CdcProgrammer {
    port: Box<dyn SerialPort>,
    sequence: u8,
}

impl CdcProgrammer {
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let mut port = serialport::new(path, baud)
            .timeout(Duration::from_millis(2000))
            .open()
            .map_err(|err| Error::Serial(format!("failed to open {path}: {err}")))?;
        port.write_data_terminal_ready(true)
            .map_err(|err| Error::Serial(format!("failed to set CDC DTR on {path}: {err}")))?;
        port.write_request_to_send(true)
            .map_err(|err| Error::Serial(format!("failed to set CDC RTS on {path}: {err}")))?;
        port.clear(serialport::ClearBuffer::All).map_err(|err| {
            Error::Serial(format!("failed to clear CDC buffers on {path}: {err}"))
        })?;
        Ok(Self { port, sequence: 0 })
    }

    fn command(&mut self, command: Command, payload: &[u8]) -> Result<Vec<u8>> {
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::Protocol(format!(
                "command payload has {} bytes, max is {MAX_PAYLOAD_SIZE}",
                payload.len()
            )));
        }

        self.sequence = self.sequence.wrapping_add(1);
        let request = Request {
            sequence: self.sequence,
            command,
            payload: payload.to_vec(),
        };
        let packet = encode_request(&request)?;
        self.port
            .write_all(&packet)
            .map_err(|err| Error::Serial(format!("CDC control write failed: {err}")))?;
        self.port
            .flush()
            .map_err(|err| Error::Serial(format!("CDC control flush failed: {err}")))?;

        let mut response_packet = [0u8; HID_PACKET_SIZE];
        let mut received = 0usize;
        while received < 4 {
            match self.port.read(&mut response_packet[received..4]) {
                Ok(0) => {}
                Ok(count) => received += count,
                Err(err) if err.kind() == ErrorKind::TimedOut => {
                    return Err(Error::Serial(format!(
                        "CDC control read timed out after {received} byte(s): {}",
                        format_hex(&response_packet[..received])
                    )));
                }
                Err(err) => {
                    return Err(Error::Serial(format!("CDC control read failed: {err}")));
                }
            }
        }
        let payload_len = response_packet[3] as usize;
        if payload_len > MAX_PAYLOAD_SIZE {
            return Err(Error::Protocol("response payload too large".to_string()));
        }
        while received < 4 + payload_len {
            match self
                .port
                .read(&mut response_packet[received..4 + payload_len])
            {
                Ok(0) => {}
                Ok(count) => received += count,
                Err(err) if err.kind() == ErrorKind::TimedOut => {
                    return Err(Error::Serial(format!(
                        "CDC control read timed out after {received} byte(s): {}",
                        format_hex(&response_packet[..received])
                    )));
                }
                Err(err) => {
                    return Err(Error::Serial(format!("CDC control read failed: {err}")));
                }
            }
        }
        let response = decode_response(&response_packet)?;
        if response.sequence != request.sequence {
            return Err(Error::Protocol(format!(
                "response sequence {} did not match request sequence {}",
                response.sequence, request.sequence
            )));
        }
        if response.status != Status::Ok {
            return Err(Error::Device(format!(
                "bridge returned status {:?}",
                response.status
            )));
        }
        Ok(response.payload)
    }
}

fn format_hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{byte:02x}");
    }
    out
}

impl ProgrammerDevice for CdcProgrammer {
    fn probe(&mut self) -> Result<ProbeInfo> {
        let response = self.command(Command::Probe, &[])?;
        let target_name = String::from_utf8_lossy(&response).to_string();
        Ok(ProbeInfo {
            bridge_name: "atu10-programmer-bridge-cdc".to_string(),
            protocol_version: 2,
            target_name,
        })
    }

    fn reset_target(&mut self) -> Result<()> {
        self.command(Command::ResetTarget, &[])?;
        Ok(())
    }

    fn read_target_id(&mut self) -> Result<u16> {
        let response = self.command(Command::ReadTargetId, &[])?;
        if response.len() != 2 {
            return Err(Error::Protocol(format!(
                "read-id response has {} bytes, expected 2",
                response.len()
            )));
        }
        Ok(u16::from_le_bytes([response[0], response[1]]))
    }

    fn begin_flash(&mut self, row_count: u32) -> Result<()> {
        self.command(Command::BeginFlash, &row_count.to_le_bytes())?;
        Ok(())
    }

    fn erase_row(&mut self, base_word_address: u32) -> Result<()> {
        self.command(Command::EraseRow, &base_word_address.to_le_bytes())?;
        Ok(())
    }

    fn write_chunk(&mut self, base_word_address: u32, offset_bytes: u8, data: &[u8]) -> Result<()> {
        let mut payload = Vec::with_capacity(5 + data.len());
        payload.extend_from_slice(&base_word_address.to_le_bytes());
        payload.push(offset_bytes);
        payload.extend_from_slice(data);
        self.command(Command::WriteChunk, &payload)?;
        Ok(())
    }

    fn commit_row(&mut self, base_word_address: u32) -> Result<()> {
        self.command(Command::CommitRow, &base_word_address.to_le_bytes())?;
        Ok(())
    }

    fn verify_range(&mut self, base_word_address: u32, expected: &[u16]) -> Result<()> {
        let digest = verify_digest(base_word_address, expected);
        let mut payload = Vec::with_capacity(10);
        payload.extend_from_slice(&base_word_address.to_le_bytes());
        payload.extend_from_slice(
            &u16::try_from(expected.len())
                .map_err(|_| Error::Device("verify range is too large".to_string()))?
                .to_le_bytes(),
        );
        payload.extend_from_slice(&digest.to_le_bytes());
        self.command(Command::VerifyRange, &payload)?;
        Ok(())
    }

    fn read_words(&mut self, base_word_address: u32, word_count: usize) -> Result<Vec<u16>> {
        let word_count = u16::try_from(word_count)
            .map_err(|_| Error::Device("read word count is too large".to_string()))?;
        let mut payload = Vec::with_capacity(6);
        payload.extend_from_slice(&base_word_address.to_le_bytes());
        payload.extend_from_slice(&word_count.to_le_bytes());
        let response = self.command(Command::ReadWords, &payload)?;
        decode_words(&response, usize::from(word_count))
    }

    fn run_target(&mut self) -> Result<()> {
        self.command(Command::RunTarget, &[])?;
        Ok(())
    }
}

impl SerialTunnel for CdcProgrammer {
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

fn decode_words(response: &[u8], word_count: usize) -> Result<Vec<u16>> {
    if response.len() != word_count * 2 {
        return Err(Error::Protocol(format!(
            "read-words response has {} bytes, expected {}",
            response.len(),
            word_count * 2
        )));
    }
    Ok(response
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x3fff)
        .collect())
}
