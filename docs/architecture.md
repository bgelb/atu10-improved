# Architecture

`atu10-improved` splits the system into a USB-side programmer bridge, an
ATU-side tuner controller, and Rust host tooling.

The programmer bridge is initially PIC16F1454-class firmware. It should enumerate
as a composite USB device:

- HID for automatable control: probe, reset, flash, verify, run.
- CDC ACM for transparent serial access to the running tuner controller.

The tuner controller is initially PIC16F18877-class firmware. The first image is
a 115200 baud hello/echo smoke target so the bridge and host loop can be proven
before tuner algorithms are built.

The Rust host tooling owns desktop parsing, planning, protocol framing, command
sequencing, and later real HID/CDC access. Fake backends exercise the no-hardware
flow without claiming to validate physical behavior.
