# Toolchain

Run:

```sh
cargo xtask doctor
```

Expected tools:

- `cargo` and `rustc` for host tooling.
- `xc8-cc` or `xc8` for PIC firmware.
- Microchip DFPs:
  - `PIC12-16F1xxx_DFP` for the PIC16F1454-class programmer bridge.
  - `PIC16F1xxxx_DFP` for the PIC16F18877-class tuner controller.
- `clang` or `gcc` for host-native C tests.
- `clang-format` for C/H formatting.
- `mdformat` for Markdown formatting.
- `pre-commit` for local hooks.

Supported development hosts should include macOS arm64, macOS x86_64, and Linux
x86_64 where Microchip ships compatible tooling. Apple Silicon systems may need
Rosetta for some Microchip components.

MPLAB X simulator is not required for this project.

The Makefiles default to Microchip's standard user pack location:
`$HOME/.mchp_packs/Microchip/...`. Override `PIC1454_DFP` or `PIC18877_DFP` when
using a different pack install location.

## GitHub CI

The no-hardware test suite runs on GitHub-hosted Linux runners with Rust and
`clang`; it does not require XC8.

Firmware compile checks do require the free MPLAB XC8 compiler and the pinned
Microchip DFPs. The CI workflow installs XC8 under `$HOME/.local/microchip` and
unpacks the DFP `.atpack` files under `$HOME/.mchp_packs/Microchip`, matching
the paths used by the firmware Makefiles. Before relying on this in a public or
organization CI environment, review Microchip's installer/license terms for your
use case.

Project-managed tools and caches should live under `$HOME/.local` or
`$HOME/.atu10-improved`. Do not require global installs into `/usr/local`,
Homebrew prefixes, or system directories.
