# Programmer Bridge Firmware

This firmware role runs on the USB-side bridge MCU, initially a PIC16F1454-class
device. Its job is to make the ATU-side controller automatable after the bridge
has been programmed once with an external ICSP programmer.

Initial intended USB shape:

- CDC ACM serial interface for a transparent 115200 baud tunnel to the tuner
  controller.
- HID control interface for probe, reset, flash, verify, and run commands.

The state-machine code is host-testable. USB enumeration, ICSP timing, MCLR
behavior, and serial buffering are hardware bring-up work and are not faked by
unit tests.
