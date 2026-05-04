use atu10_core::device::{ProgrammerDevice, F18877_DEVICE_ID};
use atu10_core::hid::HidProgrammer;
use atu10_core::serial::{serial_smoke, SystemSerialTunnel};
use std::env;

#[test]
#[ignore = "requires ATU10 hardware with the custom bridge attached"]
fn hardware_probe_read_id_and_serial_smoke() {
    let vid = env_u16("ATU10_HID_VID");
    let pid = env_u16("ATU10_HID_PID");
    let serial_port = env::var("ATU10_SERIAL_PORT").expect("ATU10_SERIAL_PORT is required");

    let mut programmer = HidProgrammer::open(vid, pid).expect("open HID programmer");
    let info = programmer.probe().expect("probe bridge");
    assert_eq!(info.protocol_version, 2);
    assert_eq!(
        programmer.read_target_id().expect("read target id"),
        F18877_DEVICE_ID
    );

    programmer.run_target().expect("run target");
    let mut serial = SystemSerialTunnel::open(&serial_port, 115200).expect("open serial tunnel");
    serial_smoke(&mut serial).expect("serial smoke");
}

fn env_u16(name: &str) -> u16 {
    let value = env::var(name).unwrap_or_else(|_| panic!("{name} is required"));
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).unwrap_or_else(|err| panic!("invalid {name}: {err}"))
    } else {
        value
            .parse()
            .unwrap_or_else(|err| panic!("invalid {name}: {err}"))
    }
}
