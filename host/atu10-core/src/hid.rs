use crate::device::{ProbeInfo, ProgrammerDevice};
use crate::flash::FlashRow;
use crate::protocol::{
    decode_response, encode_request, Command, Request, Status, HID_PACKET_SIZE, MAX_PAYLOAD_SIZE,
};
use crate::{Error, Result};
use hidapi::{HidApi, HidDevice};
use std::collections::BTreeMap;

pub const DEFAULT_TIMEOUT_MS: i32 = 2000;

pub struct HidProgrammer {
    transport: HidTransport,
}

impl HidProgrammer {
    pub fn open(vid: u16, pid: u16) -> Result<Self> {
        let api = HidApi::new().map_err(|err| Error::Device(err.to_string()))?;
        let device = api.open(vid, pid).map_err(|err| {
            Error::Device(format!("failed to open HID {vid:04x}:{pid:04x}: {err}"))
        })?;
        Ok(Self {
            transport: HidTransport::new(device),
        })
    }
}

impl ProgrammerDevice for HidProgrammer {
    fn probe(&mut self) -> Result<ProbeInfo> {
        let response = self.transport.command(Command::Probe, &[])?;
        let target_name = String::from_utf8_lossy(&response).to_string();
        Ok(ProbeInfo {
            bridge_name: "atu10-programmer-bridge".to_string(),
            protocol_version: 1,
            target_name,
        })
    }

    fn reset_target(&mut self) -> Result<()> {
        self.transport.command(Command::ResetTarget, &[])?;
        Ok(())
    }

    fn start_flash(&mut self, row_count: u32) -> Result<()> {
        self.transport
            .command(Command::StartFlash, &row_count.to_le_bytes())?;
        Ok(())
    }

    fn program_row(&mut self, row: &FlashRow) -> Result<()> {
        let mut payload = Vec::with_capacity(4 + row.data.len());
        payload.extend_from_slice(&row.base_address.to_le_bytes());
        payload.extend_from_slice(&row.data);
        self.transport.command(Command::ProgramRow, &payload)?;
        Ok(())
    }

    fn verify(&mut self, expected: &BTreeMap<u32, u8>) -> Result<()> {
        let digest = verify_digest(expected);
        self.transport
            .command(Command::Verify, &digest.to_le_bytes())?;
        Ok(())
    }

    fn run_target(&mut self) -> Result<()> {
        self.transport.command(Command::RunTarget, &[])?;
        Ok(())
    }
}

pub struct HidTransport {
    device: HidDevice,
    sequence: u8,
    timeout_ms: i32,
}

impl HidTransport {
    pub fn new(device: HidDevice) -> Self {
        Self {
            device,
            sequence: 0,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    pub fn command(&mut self, command: Command, payload: &[u8]) -> Result<Vec<u8>> {
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

        // hidapi on macOS/Linux expects a report ID byte before the report body.
        let mut report = [0u8; HID_PACKET_SIZE + 1];
        report[1..].copy_from_slice(&packet);
        self.device
            .write(&report)
            .map_err(|err| Error::Device(format!("HID write failed: {err}")))?;

        let mut response_report = [0u8; HID_PACKET_SIZE + 1];
        let len = self
            .device
            .read_timeout(&mut response_report, self.timeout_ms)
            .map_err(|err| Error::Device(format!("HID read failed: {err}")))?;
        if len == 0 {
            return Err(Error::Device(
                "timed out waiting for HID response".to_string(),
            ));
        }

        let response_packet = if len == HID_PACKET_SIZE + 1 {
            &response_report[1..]
        } else if len == HID_PACKET_SIZE {
            &response_report[..HID_PACKET_SIZE]
        } else {
            return Err(Error::Protocol(format!(
                "HID response has {len} bytes, expected {HID_PACKET_SIZE} or {}",
                HID_PACKET_SIZE + 1
            )));
        };

        let response = decode_response(response_packet)?;
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

pub fn verify_digest(bytes: &BTreeMap<u32, u8>) -> u32 {
    bytes.iter().fold(0x811c_9dc5u32, |hash, (address, value)| {
        let mixed = hash ^ address ^ u32::from(*value);
        mixed.wrapping_mul(0x0100_0193)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_digest_changes_with_address_and_data() {
        let mut first = BTreeMap::new();
        first.insert(0x10, 0xaa);
        let mut second = BTreeMap::new();
        second.insert(0x11, 0xaa);
        let mut third = BTreeMap::new();
        third.insert(0x10, 0xab);

        assert_ne!(verify_digest(&first), verify_digest(&second));
        assert_ne!(verify_digest(&first), verify_digest(&third));
    }
}
