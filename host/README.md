# Host Tools

Rust host tooling lives here. The first CLI is `atu10ctl`, and shared protocol,
Intel HEX, flash-planning, fake-device, and serial smoke-test logic lives in
`atu10-core`.

The CLI supports fake and real backend shapes:

```sh
atu10ctl probe --fake
atu10ctl flash --fake <firmware.hex>
atu10ctl serial --fake
atu10ctl probe --vid 0x1209 --pid 0xa710
atu10ctl reset --vid 0x1209 --pid 0xa710
atu10ctl flash --vid 0x1209 --pid 0xa710 <firmware.hex>
atu10ctl serial --port /dev/cu.usbmodemXXXX --baud 115200
```

The real USB HID and CDC backends are intentionally not faked in tests. The fake
backends exercise the command sequencing that can be tested without hardware;
USB enumeration, timing, and serial buffering are hardware bring-up tasks.
