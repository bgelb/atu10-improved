use crate::{Error, Result};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexImage {
    bytes: BTreeMap<u32, u8>,
}

impl HexImage {
    pub fn parse(input: &str) -> Result<Self> {
        let mut bytes = BTreeMap::new();
        let mut upper_linear = 0u32;
        let mut saw_eof = false;

        for (line_index, raw_line) in input.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if !line.starts_with(':') {
                return Err(Error::Hex(format!("line {line_number}: missing ':'")));
            }
            let record = decode_hex_bytes(&line[1..])
                .map_err(|message| Error::Hex(format!("line {line_number}: {message}")))?;
            if record.len() < 5 {
                return Err(Error::Hex(format!("line {line_number}: record too short")));
            }

            let byte_count = record[0] as usize;
            let expected_len = 4 + byte_count + 1;
            if record.len() != expected_len {
                return Err(Error::Hex(format!(
                    "line {line_number}: length says {byte_count} data bytes but record has {}",
                    record.len().saturating_sub(5)
                )));
            }

            let checksum = record
                .iter()
                .fold(0u8, |sum, value| sum.wrapping_add(*value));
            if checksum != 0 {
                return Err(Error::Hex(format!("line {line_number}: checksum mismatch")));
            }

            let offset = ((record[1] as u16) << 8) | record[2] as u16;
            let record_type = record[3];
            let data = &record[4..4 + byte_count];

            match record_type {
                0x00 => {
                    let base = upper_linear + u32::from(offset);
                    for (index, value) in data.iter().enumerate() {
                        bytes.insert(base + index as u32, *value);
                    }
                }
                0x01 => {
                    if byte_count != 0 {
                        return Err(Error::Hex(format!("line {line_number}: EOF has data")));
                    }
                    saw_eof = true;
                    break;
                }
                0x04 => {
                    if byte_count != 2 {
                        return Err(Error::Hex(format!(
                            "line {line_number}: extended linear address must have 2 bytes"
                        )));
                    }
                    upper_linear = (((data[0] as u32) << 8) | data[1] as u32) << 16;
                }
                other => {
                    return Err(Error::Hex(format!(
                        "line {line_number}: unsupported record type 0x{other:02x}"
                    )));
                }
            }
        }

        if !saw_eof {
            return Err(Error::Hex("missing EOF record".to_string()));
        }

        Ok(Self { bytes })
    }

    pub fn bytes(&self) -> &BTreeMap<u32, u8> {
        &self.bytes
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

fn decode_hex_bytes(hex: &str) -> std::result::Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("odd number of hex digits".to_string());
    }

    let mut out = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks_exact(2) {
        let high = decode_nibble(chunk[0])?;
        let low = decode_nibble(chunk[1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn decode_nibble(byte: u8) -> std::result::Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit '{}'", byte as char)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sparse_extended_linear_image() {
        let image = HexImage::parse(
            ":020000040001F9\n\
             :0400100001020304E2\n\
             :02002000AABB79\n\
             :00000001FF\n",
        )
        .unwrap();

        assert_eq!(image.bytes().get(&0x0001_0010), Some(&0x01));
        assert_eq!(image.bytes().get(&0x0001_0013), Some(&0x04));
        assert_eq!(image.bytes().get(&0x0001_0020), Some(&0xaa));
        assert_eq!(image.bytes().get(&0x0001_0021), Some(&0xbb));
        assert_eq!(image.bytes().get(&0x0001_0014), None);
    }

    #[test]
    fn rejects_bad_checksum() {
        let err = HexImage::parse(":0100000000FE\n:00000001FF\n").unwrap_err();
        assert!(err.to_string().contains("checksum"));
    }

    #[test]
    fn rejects_missing_eof() {
        let err = HexImage::parse(":0100000000FF\n").unwrap_err();
        assert!(err.to_string().contains("missing EOF"));
    }
}
