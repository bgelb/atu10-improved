# atu10-improved

`atu10-improved` is a from-scratch firmware and tooling project for the N7DDC
ATU10 autotuner, with room for ATU100 support once the board differences are
confirmed.

The first milestone is infrastructure: make it fast and automatable to iterate
on the ATU-side firmware through the existing USB-side PIC1454-class bridge. The
goal is an end-to-end loop that can flash, reset, and talk to the tuner
controller without manual unplug/replug cycles.

## End-to-end model

The system has two firmware roles and one host-side tool stack:

- `firmware/programmer-bridge/` runs on the USB-side bridge MCU, initially
  PIC16F1454. It exposes USB to the host, resets/programs the tuner controller,
  and tunnels UART traffic.
- `firmware/tuner-controller/` runs on the ATU application MCU, initially
  PIC16F18877. The first image is a UART hello/echo smoke-test image.
- `host/` contains Rust tooling. The CLI will eventually talk to the real bridge
  over USB HID for control and CDC ACM for serial.

The intended hardware workflow is:

1. Program the bridge MCU once with an external ICSP programmer.
1. Connect only the bridge USB port after that.
1. Build tuner-controller firmware with Microchip XC8.
1. Use Rust host tooling to flash the tuner controller through the bridge HID
   control path.
1. Reset or run the tuner controller through the same HID control path.
1. Use the bridge CDC ACM port as a transparent 115200 baud serial tunnel.
1. Repeat flash/reset/serial smoke cycles under automation.

The initial USB design is composite CDC ACM + HID. CDC is for the transparent
serial tunnel. HID is for probe, reset, flash, verify, and run commands. Mass
storage support is desirable later, but it is not the first automation path.

## Repository layout

- `.cargo/config.toml`: defines `cargo xtask`.
- `firmware/programmer-bridge/`: USB programmer/reset/serial bridge firmware.
- `firmware/tuner-controller/`: ATU application firmware.
- `firmware/test-support/unity/`: lightweight Unity-compatible test harness for
  host-native C tests.
- `host/atu10-core/`: shared Rust protocol, HEX, flash planning, fake device,
  and serial smoke logic.
- `host/atu10ctl/`: initial CLI shape.
- `tools/xtask/`: build, test, format, and environment-check orchestration.
- `docs/`: architecture, protocol, toolchain, testing, hardware, and bring-up
  notes.
- `AGENTS.md`: repo rules for future Codex/agent work.

Firmware directories are named by function, not part number. MCU-specific code
belongs below a firmware role, not in the role name.

## Toolchain setup

Required for normal development:

- Rust via `rustup`.
- Free Microchip MPLAB XC8 compiler for PIC firmware.
- Microchip DFPs for `PIC12-16F1xxx_DFP` and `PIC16F1xxxx_DFP`.
- `clang` or `gcc` for host-native C unit tests.
- `clang-format` for C/H formatting.
- `mdformat` for Markdown formatting.
- `pre-commit` for repository hooks.

Development should work on macOS arm64, macOS x86_64, and Linux x86_64 where
Microchip tooling supports the target devices. On Apple Silicon, Microchip
installer/tool compatibility may require Rosetta; confirm with `cargo xtask doctor`.

MPLAB X simulator is not part of the required workflow for this repo. Tests must
stay focused on host-testable logic and real hardware bring-up.

Project-managed helper installs should live under `$HOME/.local` or
`$HOME/.atu10-improved`, not system paths such as `/usr/local` or Homebrew
prefixes.

## Common commands

```sh
cargo xtask doctor
cargo xtask build-host
cargo xtask build-firmware
cargo xtask test
cargo xtask format
cargo xtask format --fix
pre-commit install
pre-commit run --all-files
```

`cargo xtask doctor` reports missing tools with actionable output. The current
workspace may not have every tool installed yet; the scaffold is designed to
make those gaps explicit.

Host CLI examples:

```sh
atu10ctl probe --fake
atu10ctl flash --fake firmware/tuner-controller/build/tuner-controller.hex
atu10ctl serial --fake
atu10ctl probe --vid 0x1209 --pid 0xa710
atu10ctl reset --vid 0x1209 --pid 0xa710
atu10ctl flash --vid 0x1209 --pid 0xa710 firmware/tuner-controller/build/tuner-controller.hex
atu10ctl serial --port /dev/cu.usbmodemXXXX --baud 115200
```

## No-hardware test strategy

Tests should pull their weight. This repo tests the logic that can fail before
hardware and avoids fake tests for electrical facts.

Meaningful no-hardware tests:

- Intel HEX parsing, including sparse records, extended linear addresses,
  checksum failures, and EOF handling.
- Flash row planning, including row alignment and fill bytes.
- HID frame encoding/decoding and malformed packets.
- Fake programmer flash/reset/verify/run sequencing through the same APIs used
  by the CLI.
- Fake CDC serial smoke path that expects the tuner hello line and echo behavior.
- Host-native C tests for portable firmware state-machine and UART app logic.

Hardware-only validation:

- USB enumeration as CDC + HID.
- CDC driver compatibility with the PIC16F1454 endpoint budget. The descriptor
  skeleton uses CDC data on EP2 and HID control on EP1, without a CDC
  notification endpoint.
- macOS/Linux/Windows driver behavior.
- CDC buffering and latency.
- PIC16F1454-to-PIC16F18877 ICSP timing.
- MCLR/reset circuit behavior.
- Real 20-100 cycle flash/reset/serial endurance.

## Current status

This is the initial infrastructure scaffold, not completed ATU firmware.

Known hardware uncertainty: the source notes mention `RC4<->RB7` and
`RC5<->RB7`, duplicating `RB7` on the tuner-controller side. Until the board is
traced or schematic-confirmed, uncertain nets must stay centralized in board
definition files.
