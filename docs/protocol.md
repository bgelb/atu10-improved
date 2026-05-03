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
- `0x10` start flash session. Payload: `u32 row_count_le`.
- `0x11` program row. Payload: `u32 base_address_le` followed by row bytes.
- `0x12` verify. Payload: `u32 stream_digest_le`.
- `0x13` run target.

The stream digest is a small FNV-1a-style guard over the byte stream:

```text
digest = 0x811c9dc5
for each programmed byte in command order:
    digest = (digest ^ address ^ value) * 0x01000193
```

This catches host/bridge command stream mismatches before target readback exists.
It is not a substitute for final hardware flash verification.

The packet shape and state-machine behavior are tested without hardware.
