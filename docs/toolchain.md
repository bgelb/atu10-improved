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

Project-managed tools and caches should live under `$HOME/.local` or
`$HOME/.atu10-improved`. Do not require global installs into `/usr/local`,
Homebrew prefixes, or system directories.
