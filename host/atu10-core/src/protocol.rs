use crate::{Error, Result};

pub const HID_PACKET_SIZE: usize = 64;
pub const MAX_PAYLOAD_SIZE: usize = HID_PACKET_SIZE - 4;
const MAGIC: u8 = 0xa7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Probe = 0x01,
    ResetTarget = 0x02,
    StartFlash = 0x10,
    ProgramRow = 0x11,
    Verify = 0x12,
    RunTarget = 0x13,
}

impl Command {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::Probe),
            0x02 => Ok(Self::ResetTarget),
            0x10 => Ok(Self::StartFlash),
            0x11 => Ok(Self::ProgramRow),
            0x12 => Ok(Self::Verify),
            0x13 => Ok(Self::RunTarget),
            _ => Err(Error::Protocol(format!("unknown command 0x{value:02x}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Ok = 0x00,
    Busy = 0x01,
    BadRequest = 0x80,
    VerifyFailed = 0x81,
    HardwareFault = 0x82,
}

impl Status {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Ok),
            0x01 => Ok(Self::Busy),
            0x80 => Ok(Self::BadRequest),
            0x81 => Ok(Self::VerifyFailed),
            0x82 => Ok(Self::HardwareFault),
            _ => Err(Error::Protocol(format!("unknown status 0x{value:02x}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub sequence: u8,
    pub command: Command,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub sequence: u8,
    pub status: Status,
    pub payload: Vec<u8>,
}

pub fn encode_request(request: &Request) -> Result<[u8; HID_PACKET_SIZE]> {
    if request.payload.len() > MAX_PAYLOAD_SIZE {
        return Err(Error::Protocol(format!(
            "payload has {} bytes, max is {MAX_PAYLOAD_SIZE}",
            request.payload.len()
        )));
    }

    let mut packet = [0u8; HID_PACKET_SIZE];
    packet[0] = MAGIC;
    packet[1] = request.sequence;
    packet[2] = request.command as u8;
    packet[3] = request.payload.len() as u8;
    packet[4..4 + request.payload.len()].copy_from_slice(&request.payload);
    Ok(packet)
}

pub fn decode_request(packet: &[u8]) -> Result<Request> {
    if packet.len() != HID_PACKET_SIZE {
        return Err(Error::Protocol(format!(
            "request packet has {} bytes, expected {HID_PACKET_SIZE}",
            packet.len()
        )));
    }
    if packet[0] != MAGIC {
        return Err(Error::Protocol("bad request magic".to_string()));
    }
    let payload_len = packet[3] as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(Error::Protocol("request payload too large".to_string()));
    }

    Ok(Request {
        sequence: packet[1],
        command: Command::from_u8(packet[2])?,
        payload: packet[4..4 + payload_len].to_vec(),
    })
}

pub fn encode_response(response: &Response) -> Result<[u8; HID_PACKET_SIZE]> {
    if response.payload.len() > MAX_PAYLOAD_SIZE {
        return Err(Error::Protocol(format!(
            "payload has {} bytes, max is {MAX_PAYLOAD_SIZE}",
            response.payload.len()
        )));
    }

    let mut packet = [0u8; HID_PACKET_SIZE];
    packet[0] = MAGIC;
    packet[1] = response.sequence;
    packet[2] = response.status as u8;
    packet[3] = response.payload.len() as u8;
    packet[4..4 + response.payload.len()].copy_from_slice(&response.payload);
    Ok(packet)
}

pub fn decode_response(packet: &[u8]) -> Result<Response> {
    if packet.len() != HID_PACKET_SIZE {
        return Err(Error::Protocol(format!(
            "response packet has {} bytes, expected {HID_PACKET_SIZE}",
            packet.len()
        )));
    }
    if packet[0] != MAGIC {
        return Err(Error::Protocol("bad response magic".to_string()));
    }
    let payload_len = packet[3] as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(Error::Protocol("response payload too large".to_string()));
    }

    Ok(Response {
        sequence: packet[1],
        status: Status::from_u8(packet[2])?,
        payload: packet[4..4 + payload_len].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_hid_packet() {
        let request = Request {
            sequence: 42,
            command: Command::ProgramRow,
            payload: vec![1, 2, 3],
        };

        let packet = encode_request(&request).unwrap();
        assert_eq!(decode_request(&packet).unwrap(), request);
    }

    #[test]
    fn response_round_trips_through_hid_packet() {
        let response = Response {
            sequence: 7,
            status: Status::Ok,
            payload: b"ATU10".to_vec(),
        };

        let packet = encode_response(&response).unwrap();
        assert_eq!(decode_response(&packet).unwrap(), response);
    }

    #[test]
    fn rejects_malformed_response() {
        let mut packet = [0u8; HID_PACKET_SIZE];
        packet[0] = 0x55;
        assert!(decode_response(&packet).is_err());

        packet[0] = MAGIC;
        packet[2] = 0xff;
        assert!(decode_response(&packet).is_err());
    }

    #[test]
    fn rejects_oversized_payload() {
        let request = Request {
            sequence: 1,
            command: Command::Probe,
            payload: vec![0; MAX_PAYLOAD_SIZE + 1],
        };

        assert!(encode_request(&request).is_err());
    }
}
