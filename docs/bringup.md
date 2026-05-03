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
