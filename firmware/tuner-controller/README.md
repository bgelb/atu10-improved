# Tuner Controller Firmware

This firmware role runs on the ATU application MCU, initially the PIC16F18877
on the ATU10 board. The first milestone is a minimal UART smoke image, not the
complete tuner algorithm.

The image should eventually:

- Configure oscillator, PPS, and EUSART for 115200 baud.
- Print `ATU10-IMPROVED READY 115200`.
- Echo received bytes so the host can prove flash, reset, and serial tunnel
  behavior end to end.

Pin assignments are intentionally centralized in `boards/atu10/board.h` until
the duplicated `RB7` note from the source material is resolved.
