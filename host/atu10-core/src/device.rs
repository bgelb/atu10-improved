use crate::flash::{FlashPlan, FlashRow};
use crate::hex::HexImage;
use crate::{Error, Result};
use std::collections::BTreeMap;

pub const DEFAULT_ROW_SIZE: usize = 32;
pub const FLASH_FILL: u8 = 0xff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeInfo {
    pub bridge_name: String,
    pub protocol_version: u16,
    pub target_name: String,
}

pub trait ProgrammerDevice {
    fn probe(&mut self) -> Result<ProbeInfo>;
    fn reset_target(&mut self) -> Result<()>;
    fn start_flash(&mut self, row_count: u32) -> Result<()>;
    fn program_row(&mut self, row: &FlashRow) -> Result<()>;
    fn verify(&mut self, expected: &BTreeMap<u32, u8>) -> Result<()>;
    fn run_target(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    Probe,
    ResetTarget,
    StartFlash { row_count: u32 },
    ProgramRow { base_address: u32, len: usize },
    Verify { byte_count: usize },
    RunTarget,
}

#[derive(Debug, Clone)]
pub struct FakeProgrammer {
    info: ProbeInfo,
    memory: BTreeMap<u32, u8>,
    events: Vec<DeviceEvent>,
    fail_verify: bool,
}

impl Default for FakeProgrammer {
    fn default() -> Self {
        Self {
            info: ProbeInfo {
                bridge_name: "fake-programmer-bridge".to_string(),
                protocol_version: 1,
                target_name: "fake-pic16f18877".to_string(),
            },
            memory: BTreeMap::new(),
            events: Vec::new(),
            fail_verify: false,
        }
    }
}

impl FakeProgrammer {
    pub fn events(&self) -> &[DeviceEvent] {
        &self.events
    }

    pub fn memory(&self) -> &BTreeMap<u32, u8> {
        &self.memory
    }

    pub fn set_fail_verify(&mut self, fail_verify: bool) {
        self.fail_verify = fail_verify;
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

    fn start_flash(&mut self, row_count: u32) -> Result<()> {
        self.events.push(DeviceEvent::StartFlash { row_count });
        self.memory.clear();
        Ok(())
    }

    fn program_row(&mut self, row: &FlashRow) -> Result<()> {
        self.events.push(DeviceEvent::ProgramRow {
            base_address: row.base_address,
            len: row.data.len(),
        });
        for (index, value) in row.data.iter().enumerate() {
            self.memory.insert(row.base_address + index as u32, *value);
        }
        Ok(())
    }

    fn verify(&mut self, expected: &BTreeMap<u32, u8>) -> Result<()> {
        self.events.push(DeviceEvent::Verify {
            byte_count: expected.len(),
        });
        if self.fail_verify || &self.memory != expected {
            return Err(Error::Device("fake verify failed".to_string()));
        }
        Ok(())
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
    let plan = FlashPlan::from_image(image, DEFAULT_ROW_SIZE, FLASH_FILL)?;

    device.probe()?;
    device.reset_target()?;
    device.start_flash(plan.rows.len() as u32)?;
    for row in &plan.rows {
        device.program_row(row)?;
    }
    device.verify(&plan.verify_image())?;
    device.run_target()?;

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::HexImage;

    #[test]
    fn fake_flash_pipeline_uses_expected_sequence() {
        let image = HexImage::parse(
            ":0400000001020304F2\n\
             :02003000AABB69\n\
             :00000001FF\n",
        )
        .unwrap();
        let mut device = FakeProgrammer::default();

        let plan = flash_image(&mut device, &image).unwrap();

        assert_eq!(
            device.events(),
            &[
                DeviceEvent::Probe,
                DeviceEvent::ResetTarget,
                DeviceEvent::StartFlash {
                    row_count: plan.rows.len() as u32
                },
                DeviceEvent::ProgramRow {
                    base_address: 0,
                    len: DEFAULT_ROW_SIZE
                },
                DeviceEvent::ProgramRow {
                    base_address: 0x20,
                    len: DEFAULT_ROW_SIZE
                },
                DeviceEvent::Verify {
                    byte_count: DEFAULT_ROW_SIZE * 2
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
}
