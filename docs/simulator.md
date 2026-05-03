# Simulator Notes

MPLAB X simulator is not required or expected for this project.

The no-hardware suite should stay focused on portable logic that can be tested
with Rust tests or host-native C tests. Real bridge and tuner-controller
behavior must be proven on the board during bring-up.

Do not add required simulator tests for:

- USB enumeration.
- Composite CDC + HID behavior.
- Host operating-system driver behavior.
- CDC latency/buffering.
- ICSP electrical timing.
- MCLR circuit behavior.
- Oscillator/PPS/SFR register setup.
