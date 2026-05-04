# Protocol

The control protocol is a 64-byte HID packet shape shared by host and
programmer-bridge firmware. Protocol version 2 preserves target configuration
words by default: normal flash commands operate on PIC16F18877 program flash
rows only.

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

Commands:

- `0x01` probe.
- `0x02` reset target.
- `0x03` read target ID. Response payload: `u16 device_id_le`; PIC16F18877 is
  `0x3075`.
- `0x10` begin flash session. Payload: `u32 row_count_le`.
- `0x11` erase row. Payload: `u32 base_word_address_le`.
- `0x12` write chunk. Payload: `u32 base_word_address_le`, `u8 offset_bytes`,
  then packed row bytes.
- `0x13` commit row. Payload: `u32 base_word_address_le`.
- `0x14` verify range. Payload: `u32 base_word_address_le`,
  `u16 word_count_le`, `u32 digest_le`.
- `0x15` run target.
- `0x16` read words. Payload: `u32 base_word_address_le`,
  `u16 word_count_le`. Response payload: little-endian 14-bit words. This is
  read-only and is used for diagnostics/readback; it does not enable config
  writes.

The verify digest is a small FNV-1a-style guard over 14-bit words:

```text
digest = 0x811c9dc5
for each verified word in address order:
    digest = (digest ^ word_address ^ (word & 0x3fff)) * 0x01000193
```

Rows are 32 14-bit words. The host sends each row as little-endian packed words
and splits row data across HID packets because a full row is larger than one
request payload.

The packet shape and state-machine behavior are tested without hardware.
