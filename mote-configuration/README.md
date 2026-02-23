# mote-configuration

Web app for configuring and debugging Mote.

Provides WASM bindings for the mote sansio driver via a rust library (`./src/lib.rs`). A Svelte app (`./app/`) uses the WASM to read and write configuration values on the robot over UART.
