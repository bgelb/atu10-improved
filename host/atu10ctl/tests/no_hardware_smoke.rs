use atu10_core::device::{flash_image, DeviceEvent, FakeProgrammer};
use atu10_core::flash::F18877_ROW_WORDS;
use atu10_core::hex::HexImage;
use atu10_core::serial::{serial_smoke, FakeSerialTunnel};

#[test]
fn flash_reset_run_pipeline_works_against_fake_bridge() {
    let image = HexImage::parse(
        ":100000000C945C000C946E000C946E000C946E00CA\n\
         :08004000112233445566778854\n\
         :00000001FF\n",
    )
    .unwrap();
    let mut bridge = FakeProgrammer::default();

    let plan = flash_image(&mut bridge, &image).unwrap();

    assert_eq!(plan.rows.len(), 2);
    assert_eq!(bridge.events().first(), Some(&DeviceEvent::Probe));
    assert!(bridge
        .events()
        .contains(&DeviceEvent::BeginFlash { row_count: 2 }));
    assert!(bridge.events().contains(&DeviceEvent::VerifyRange {
        base_word_address: 0,
        word_count: F18877_ROW_WORDS
    }));
    assert_eq!(bridge.events().last(), Some(&DeviceEvent::RunTarget));
}

#[test]
fn serial_smoke_uses_same_shape_as_hardware_bringup() {
    let mut serial = FakeSerialTunnel::default();
    serial_smoke(&mut serial).unwrap();
}
