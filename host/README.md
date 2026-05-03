# Host Tools

Rust host tooling lives here. The first CLI is `atu10ctl`, and shared protocol,
Intel HEX, flash-planning, fake-device, and serial smoke-test logic lives in
`atu10-core`.

The real USB HID and CDC backends are intentionally not faked in this first
scaffold. The fake backends exercise the command sequencing that can be tested
without hardware; USB enumeration, timing, and serial buffering are hardware
bring-up tasks.
