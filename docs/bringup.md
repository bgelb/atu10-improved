# Bring-up

Suggested hardware bring-up order:

1. Program the programmer bridge with external ICSP.
1. Confirm USB enumeration.
1. Confirm the HID interface opens by VID/PID.
1. Confirm `atu10ctl probe --vid 0x1209 --pid 0xa710`.
1. Confirm `atu10ctl reset --vid 0x1209 --pid 0xa710` toggles target MCLR.
1. Confirm CDC serial tunnel at 115200 with `atu10ctl serial --port <path>`.
1. Confirm target Program/Verify entry and device ID read.
1. Program and verify one row.
1. Flash the tuner-controller smoke image.
1. Confirm hello/echo serial smoke behavior.
1. Run 20-100 automated flash/reset/serial cycles.

Keep logic-analyzer captures and deviations in this document or nearby dated
notes as the hardware path becomes real.

## PICkit3 external ICSP notes

A Microchip PICkit3 can be used from Linux as an external ICSP programmer for
the USB-side PIC16F1454 bridge MCU without installing MPLAB X. This is an
optional bring-up path, not a project-managed dependency.

Known-good host-side setup:

- PICkit3 USB ID: `04d8:900a`.

- Tool: `pk2cmd` / PICkit 2 minus, tested with executable version `1.27.01`.

- Device file tested: `2.64.33`.

- PICkit3 scripting firmware tested: `2.32.02`.

- PIC16F1454 target ID read through ICSP:

  ```text
  Device ID = 3020
  Revision  = 1005
  Device Name = PIC16F1454
  ```

Typical non-destructive checks:

```sh
pk2cmd -S#
pk2cmd -?Ppic16f145
pk2cmd -PPIC16F1454 -I -R
```

The `-R` option releases `/MCLR` after the operation. Avoid erase/program
commands until the target wiring and image are intentional.

The project helper is deliberately conservative:

```sh
cargo xtask flash-bridge
```

builds the bridge and saves a PIC16F1454 config readback under `target/xtask/`,
then refuses to program because `pk2cmd` cannot prove config-word preservation
for a full erase/program cycle. Use `--allow-config-write` only when replacing
the bridge firmware and accepting the config-word risk is intentional.

After a custom bridge is installed and enumerating as `0x1209:0xa710`, the
intended F18877 smoke path is:

```sh
cargo xtask build-firmware
atu10ctl read-id --vid 0x1209 --pid 0xa710
atu10ctl flash --vid 0x1209 --pid 0xa710 firmware/tuner-controller/build/tuner-controller.hex
atu10ctl serial --port /dev/ttyACM0 --baud 115200
cargo xtask test-hardware --vid 0x1209 --pid 0xa710 --serial /dev/ttyACM0
```

Current implementation note: the repository has the HID protocol, descriptors,
F18877 ICSP layer, and CDC/serial host tooling, but not a complete PIC16F1454
USB service loop yet. HID packets must be wired to `pb_handle_request()` and CDC
EP2 must be wired to the target UART before the custom bridge can replace the
stock firmware for end-to-end use.

### Linux USB permissions

The PICkit3 needs a udev rule so normal users in `plugdev` can access it.
Install a rule such as:

```sh
sudo tee /etc/udev/rules.d/60-pickit.rules >/dev/null <<'EOF'
# PICkit 2
ATTRS{idVendor}=="04d8", ATTRS{idProduct}=="0033", MODE="0660", GROUP="plugdev"
# PICkit 3
ATTRS{idVendor}=="04d8", ATTRS{idProduct}=="900a", MODE="0660", GROUP="plugdev"
# PKOB
ATTRS{idVendor}=="04d8", ATTRS{idProduct}=="8107", MODE="0660", GROUP="plugdev"
EOF
sudo udevadm control --reload-rules
sudo udevadm trigger --attr-match=idVendor=04d8 --attr-match=idProduct=900a
```

Then replug the PICkit3 and confirm the node is owned by `root:plugdev`:

```sh
lsusb | grep -i pickit
ls -l /dev/bus/usb/<bus>/<device>
groups
```

The user running `pk2cmd` must be in `plugdev`; log out and back in if group
membership was just changed. On Ubuntu 24.04, the `pk2cmd` AppImage needed the
FUSE 2 compatibility library:

```sh
sudo apt-get install libfuse2t64
```

If FUSE is unavailable, the AppImage can also be extracted and the contained
`usr/bin/pk2cmd` run directly from a user-writable tools directory. Keep this
outside the repository, for example under `$HOME/.atu10-improved/tools`.

### PICkit3 firmware mode

PICkit3 units may arrive in MPLAB-mode firmware. In that state `pk2cmd -S#`
can enumerate the unit, but reports `<mplab mode>` and cannot program targets.
`pk2cmd` needs the PICkit3 scripting firmware.

One working bootstrap path is:

1. Use the Linux port of Microchip's PICkit3 Programmer Application only to
   switch the PICkit3 from MPLAB firmware to scripting firmware.
1. Then use `pk2cmd` as the day-to-day CLI programmer.

The bootstrap used during bring-up loaded:

- Bootloader: `PK3BLV011405.hex`.
- Scripting OS: `PK3OSV023202.hex`.

Before switching modes, the observed PICkit3 was:

```text
Serial: BUR195068601
MPLAB application version: 9.2.6.2
```

The old MPLAB application image was not read back; the available protocol path
exposes version query, mode switching, and firmware write, not a full MPLAB app
dump. To restore MPLAB use later, revert the PICkit3 to bootloader/MPLAB mode
with a compatible PICkit3 programmer application path and let MPLAB X/IPE reload
Microchip's firmware.

### Original N7DDC bridge USB enumeration

With the original N7DDC bridge/loader firmware still on the PIC16F1454, the
same board enumerated on Linux as:

```text
04d8:0057 Microchip Composite Device
serial: 123456789099
```

Observed interfaces:

- `/dev/ttyACM0`: CDC ACM serial.
- `/dev/sda`: 2 MiB USB mass storage, FAT12, label `ATU-10 Prog`.

This confirms that the stock bridge firmware is visible to Linux independently
of the PICkit3 ICSP connection.
