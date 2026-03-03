# mote-firmware

Embassy firmware for the RP2530 that runs mote.

## Build and Deploy

Requirements:
* [cargo](curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh) (rust package manager)
* [just](https://just.systems/man/en/) (task runner)
* [probe-rs](https://probe.rs/docs/getting-started/installation/) (flasher / debugger)

From this folder, run

```
just deploy
```
