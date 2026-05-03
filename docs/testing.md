# Testing

The test strategy is intentionally split between no-hardware checks and hardware
bring-up checks.

No-hardware checks should cover:

- Intel HEX parsing.
- Flash row planning.
- HID framing and error handling.
- Fake programmer probe/reset/flash/verify/run sequencing.
- Fake serial hello/echo smoke behavior.
- Portable C firmware state machines with Unity-style tests.

No-hardware checks should not claim to cover:

- USB enumeration.
- Host driver behavior.
- CDC buffering and latency.
- ICSP electrical timing.
- MCLR/reset circuit behavior.
- Direct SFR writes or oscillator constants.

Run the no-hardware suite with:

```sh
cargo xtask test
```

Use XC8 compile checks for firmware build health:

```sh
cargo xtask build-firmware
```

MPLAB X simulator is not required or expected. Keep no-hardware tests focused on
portable logic, and treat the real board as the source of truth for USB,
programming timing, reset behavior, and serial behavior.
