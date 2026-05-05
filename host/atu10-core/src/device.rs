use crate::flash::{FlashPlan, FlashRow, F18877_ROW_WORDS};
use crate::hex::HexImage;
use crate::{Error, Result};
use std::collections::BTreeMap;

pub const F18877_DEVICE_ID: u16 = 0x3075;
pub const MAX_HID_CHUNK_BYTES: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeInfo {
    pub bridge_name: String,
    pub protocol_version: u16,
    pub target_name: String,
}

pub trait ProgrammerDevice {
    fn probe(&mut self) -> Result<ProbeInfo>;
    fn reset_target(&mut self) -> Result<()>;
    fn read_target_id(&mut self) -> Result<u16>;
    fn begin_flash(&mut self, row_count: u32) -> Result<()>;
    fn erase_row(&mut self, base_word_address: u32) -> Result<()>;
    fn write_chunk(&mut self, base_word_address: u32, offset_bytes: u8, data: &[u8]) -> Result<()>;
    fn commit_row(&mut self, base_word_address: u32) -> Result<()>;
    fn verify_range(&mut self, base_word_address: u32, expected: &[u16]) -> Result<()>;
    fn read_words(&mut self, base_word_address: u32, word_count: usize) -> Result<Vec<u16>>;
    fn run_target(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    Probe,
    ResetTarget,
    ReadTargetId,
    BeginFlash {
        row_count: u32,
    },
    EraseRow {
        base_word_address: u32,
    },
    WriteChunk {
        base_word_address: u32,
        offset_bytes: u8,
        len: usize,
    },
    CommitRow {
        base_word_address: u32,
    },
    VerifyRange {
        base_word_address: u32,
        word_count: usize,
    },
    ReadWords {
        base_word_address: u32,
        word_count: usize,
    },
    RunTarget,
}

#[derive(Debug, Clone)]
pub struct FakeProgrammer {
    info: ProbeInfo,
    memory: BTreeMap<u32, u16>,
    row_buffers: BTreeMap<u32, Vec<u8>>,
    events: Vec<DeviceEvent>,
    target_id: u16,
    fail_verify: bool,
}

impl Default for FakeProgrammer {
    fn default() -> Self {
        Self {
            info: ProbeInfo {
                bridge_name: "fake-programmer-bridge".to_string(),
                protocol_version: 2,
                target_name: "fake-pic16f18877".to_string(),
            },
            memory: BTreeMap::new(),
            row_buffers: BTreeMap::new(),
            events: Vec::new(),
            target_id: F18877_DEVICE_ID,
            fail_verify: false,
        }
    }
}

impl FakeProgrammer {
    pub fn events(&self) -> &[DeviceEvent] {
        &self.events
    }

    pub fn memory(&self) -> &BTreeMap<u32, u16> {
        &self.memory
    }

    pub fn set_fail_verify(&mut self, fail_verify: bool) {
        self.fail_verify = fail_verify;
    }

    pub fn set_target_id(&mut self, target_id: u16) {
        self.target_id = target_id;
    }
}

impl ProgrammerDevice for FakeProgrammer {
    fn probe(&mut self) -> Result<ProbeInfo> {
        self.events.push(DeviceEvent::Probe);
        Ok(self.info.clone())
    }

    fn reset_target(&mut self) -> Result<()> {
        self.events.push(DeviceEvent::ResetTarget);
        Ok(())
    }

    fn read_target_id(&mut self) -> Result<u16> {
        self.events.push(DeviceEvent::ReadTargetId);
        Ok(self.target_id)
    }

    fn begin_flash(&mut self, row_count: u32) -> Result<()> {
        self.events.push(DeviceEvent::BeginFlash { row_count });
        self.row_buffers.clear();
        Ok(())
    }

    fn erase_row(&mut self, base_word_address: u32) -> Result<()> {
        self.events
            .push(DeviceEvent::EraseRow { base_word_address });
        for index in 0..F18877_ROW_WORDS {
            self.memory
                .remove(&(base_word_address + u32::try_from(index).unwrap()));
        }
        self.row_buffers
            .insert(base_word_address, vec![0xff; F18877_ROW_WORDS * 2]);
        Ok(())
    }

    fn write_chunk(&mut self, base_word_address: u32, offset_bytes: u8, data: &[u8]) -> Result<()> {
        self.events.push(DeviceEvent::WriteChunk {
            base_word_address,
            offset_bytes,
            len: data.len(),
        });
        let row = self
            .row_buffers
            .get_mut(&base_word_address)
            .ok_or_else(|| Error::Device("fake row was not erased before write".to_string()))?;
        let start = usize::from(offset_bytes);
        let end = start + data.len();
        if end > row.len() {
            return Err(Error::Device("fake row chunk exceeds row size".to_string()));
        }
        row[start..end].copy_from_slice(data);
        Ok(())
    }

    fn commit_row(&mut self, base_word_address: u32) -> Result<()> {
        self.events
            .push(DeviceEvent::CommitRow { base_word_address });
        let row = self
            .row_buffers
            .remove(&base_word_address)
            .ok_or_else(|| Error::Device("fake row was not buffered".to_string()))?;
        for (index, bytes) in row.chunks_exact(2).enumerate() {
            let word = (u16::from(bytes[1]) << 8) | u16::from(bytes[0]);
            self.memory.insert(
                base_word_address + u32::try_from(index).unwrap(),
                word & 0x3fff,
            );
        }
        Ok(())
    }

    fn verify_range(&mut self, base_word_address: u32, expected: &[u16]) -> Result<()> {
        self.events.push(DeviceEvent::VerifyRange {
            base_word_address,
            word_count: expected.len(),
        });
        if self.fail_verify {
            return Err(Error::Device("fake verify failed".to_string()));
        }
        for (index, expected_word) in expected.iter().enumerate() {
            let address = base_word_address + u32::try_from(index).unwrap();
            if self.memory.get(&address).copied().unwrap_or(0x3fff) != (*expected_word & 0x3fff) {
                return Err(Error::Device("fake verify failed".to_string()));
            }
        }
        Ok(())
    }

    fn read_words(&mut self, base_word_address: u32, word_count: usize) -> Result<Vec<u16>> {
        self.events.push(DeviceEvent::ReadWords {
            base_word_address,
            word_count,
        });
        Ok((0..word_count)
            .map(|index| {
                self.memory
                    .get(&(base_word_address + u32::try_from(index).unwrap()))
                    .copied()
                    .unwrap_or(0x3fff)
            })
            .collect())
    }

    fn run_target(&mut self) -> Result<()> {
        self.events.push(DeviceEvent::RunTarget);
        Ok(())
    }
}

pub fn flash_image<D: ProgrammerDevice + ?Sized>(
    device: &mut D,
    image: &HexImage,
) -> Result<FlashPlan> {
    let plan = FlashPlan::f18877_from_image(image)?;

    device.probe()?;
    device.reset_target()?;
    let target_id = device.read_target_id()?;
    if target_id != F18877_DEVICE_ID {
        return Err(Error::Device(format!(
            "unexpected target device id 0x{target_id:04x}; expected PIC16F18877 0x{F18877_DEVICE_ID:04x}"
        )));
    }
    device.begin_flash(plan.rows.len() as u32)?;
    for row in &plan.rows {
        program_row(device, row)?;
    }
    device.run_target()?;

    Ok(plan)
}

fn program_row<D: ProgrammerDevice + ?Sized>(device: &mut D, row: &FlashRow) -> Result<()> {
    device.erase_row(row.base_word_address)?;
    let packed = row.packed_bytes();
    for (offset, chunk) in packed.chunks(MAX_HID_CHUNK_BYTES).enumerate() {
        let offset_bytes = u8::try_from(offset * MAX_HID_CHUNK_BYTES)
            .map_err(|_| Error::Device("row chunk offset exceeded u8".to_string()))?;
        device.write_chunk(row.base_word_address, offset_bytes, chunk)?;
    }
    device.commit_row(row.base_word_address)?;
    device.verify_range(row.base_word_address, &row.words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::HexImage;

    #[test]
    fn fake_flash_pipeline_uses_expected_sequence() {
        let image = HexImage::parse(
            ":0400000001020304F2\n\
             :02004000AABB59\n\
             :00000001FF\n",
        )
        .unwrap();
        let mut device = FakeProgrammer::default();

        let plan = flash_image(&mut device, &image).unwrap();

        assert_eq!(plan.rows.len(), 2);
        assert_eq!(
            device.events(),
            &[
                DeviceEvent::Probe,
                DeviceEvent::ResetTarget,
                DeviceEvent::ReadTargetId,
                DeviceEvent::BeginFlash { row_count: 2 },
                DeviceEvent::EraseRow {
                    base_word_address: 0
                },
                DeviceEvent::WriteChunk {
                    base_word_address: 0,
                    offset_bytes: 0,
                    len: MAX_HID_CHUNK_BYTES
                },
                DeviceEvent::WriteChunk {
                    base_word_address: 0,
                    offset_bytes: MAX_HID_CHUNK_BYTES as u8,
                    len: F18877_ROW_WORDS * 2 - MAX_HID_CHUNK_BYTES
                },
                DeviceEvent::CommitRow {
                    base_word_address: 0
                },
                DeviceEvent::VerifyRange {
                    base_word_address: 0,
                    word_count: F18877_ROW_WORDS
                },
                DeviceEvent::EraseRow {
                    base_word_address: 0x20
                },
                DeviceEvent::WriteChunk {
                    base_word_address: 0x20,
                    offset_bytes: 0,
                    len: MAX_HID_CHUNK_BYTES
                },
                DeviceEvent::WriteChunk {
                    base_word_address: 0x20,
                    offset_bytes: MAX_HID_CHUNK_BYTES as u8,
                    len: F18877_ROW_WORDS * 2 - MAX_HID_CHUNK_BYTES
                },
                DeviceEvent::CommitRow {
                    base_word_address: 0x20
                },
                DeviceEvent::VerifyRange {
                    base_word_address: 0x20,
                    word_count: F18877_ROW_WORDS
                },
                DeviceEvent::RunTarget
            ]
        );
    }

    #[test]
    fn fake_verify_failure_surfaces_as_error() {
        let image = HexImage::parse(":0400000001020304F2\n:00000001FF\n").unwrap();
        let mut device = FakeProgrammer::default();
        device.set_fail_verify(true);

        let err = flash_image(&mut device, &image).unwrap_err();
        assert!(err.to_string().contains("verify"));
    }

    #[test]
    fn target_id_mismatch_is_rejected_before_flash() {
        let image = HexImage::parse(":0400000001020304F2\n:00000001FF\n").unwrap();
        let mut device = FakeProgrammer::default();
        device.set_target_id(0x3020);

        let err = flash_image(&mut device, &image).unwrap_err();
        assert!(err.to_string().contains("device id"));
    }
}
