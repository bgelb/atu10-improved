# AGENTS

Rules for Codex and other coding agents working in this repository.

## Core rules

- Firmware directories are named by chip function, not part number.
- PIC firmware must be C built with the free Microchip MPLAB XC8 compiler.
- Host-side tools must be Rust.
- Use `cargo xtask` as the build, test, format, and doctor entrypoint.
- Keep programmer-bridge, tuner-controller, and host tooling separated.
- Keep board pins and uncertain hardware mappings centralized in board
  definition files.
- Do not install project-managed tools into system paths. Prefer `$HOME/.local`
  or `$HOME/.atu10-improved`.

## Testing rules

- Tests must pull their weight.
- Test pure logic, protocol framing, flash planning, command sequencing, and
  fake-device flows without hardware.
- Do not pretend to test USB enumeration, CDC buffering, ICSP electrical timing,
  MCLR circuit behavior, oscillator register values, or SFR writes with fake
  unit tests.
- C unit tests use the Unity-style harness under `firmware/test-support/unity`
  and are run through `cargo xtask test`.
- Portable firmware logic should be isolated from MCU/HAL code so it can be
  compiled natively with clang/gcc.
- XC8 compile checks are the guard for real PIC headers, config bits, register
  names, and device build health.

## Formatting and linting

- Rust: `cargo fmt -- --check` and
  `cargo clippy --all-targets --all-features -- -D warnings`.
- Markdown: `mdformat --check`.
- C/H: `clang-format --dry-run --Werror`.
- Do not enable global `clang-tidy` until the repo has enough XC8-specific
  suppressions to avoid noisy false positives.

## Implementation guidance

- Prefer small, mockable state machines at firmware/host boundaries.
- Keep host protocol types shared by CLI and tests.
- Add real hardware access behind traits/interfaces so fake no-hardware tests
  continue to exercise the public workflow.
- Document hardware unknowns instead of silently guessing them.
