# Unity Test Support

This directory contains a tiny Unity-compatible subset so the initial firmware
C tests run without a network fetch. Keep tests to the macros present here until
the project decides to vendor upstream ThrowTheSwitch Unity.

When the test surface grows, replace this subset with a pinned upstream Unity
copy and keep `cargo xtask test` as the entrypoint.
