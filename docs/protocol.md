# Protocol

The first control protocol is a 64-byte HID packet shape shared by host and
programmer-bridge firmware.

Request packet:

- Byte 0: magic `0xa7`.
- Byte 1: sequence number.
- Byte 2: command.
- Byte 3: payload length, maximum 60.
- Bytes 4-63: payload.

Response packet:

- Byte 0: magic `0xa7`.
- Byte 1: matching sequence number.
- Byte 2: status.
- Byte 3: payload length, maximum 60.
- Bytes 4-63: payload.

Initial commands:

- `0x01` probe.
- `0x02` reset target.
- `0x10` start flash session.
- `0x11` program row.
- `0x12` verify.
- `0x13` run target.

The real USB transport is not implemented in this scaffold. The packet shape and
state-machine behavior are tested without hardware.
