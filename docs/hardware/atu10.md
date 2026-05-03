# ATU10 Hardware Notes

Initial target board: N7DDC ATU10.

Known assumptions:

- USB-side bridge MCU is PIC16F1454-class.
- ATU-side controller MCU is PIC16F18877-class.
- Bridge firmware can be installed once with an external ICSP programmer.
- After bridge installation, F18877 flash/reset/serial access should happen
  through the bridge USB port.
- Serial tunnel target is 115200 baud.

Known uncertainty:

- Source notes mention `RC4<->RB7` and `RC5<->RB7`, duplicating `RB7` on the
  tuner-controller side. Do not hard-code this outside board definition files
  until the schematic or board trace confirms the real nets.

Hardware-only bring-up checklist:

- Bridge enumerates as CDC + HID.
- CDC enumerates on target host operating systems despite the descriptor
  skeleton omitting the optional CDC notification endpoint for endpoint-budget
  reasons.
- HID probe responds.
- HID reset toggles F18877 MCLR correctly.
- CDC tunnel reaches the F18877 UART at 115200.
- Bridge can enter F18877 Program/Verify mode.
- Host can read target device ID.
- Host can erase/program/verify one row.
- Host can flash a full HEX image and reset/run it.
- 20-100 flash/reset/serial cycles complete without unplugging.
