# Development Environment

## DevContainer

DevContainer support coming in [#13](https://github.com/empriselab/mote-core/issues/13). DevContainers do not support USB passthrough (outside of Linux), so you'll need to follow the local install directions if you wish to develop firmware.

## Local Install

Linux and MacOS are officially supported development platforms. See the [Windows section](#windows) for tips on attempting to develop using Windows.

Install the following tools:

| Tool | Purpose | Installation Method | 
|---|---|---|
| rust | cargo (package manager), rustc (compiler), rust-analyzer (language server) | [https://rustup.rs/](https://rustup.rs/) |
| just | task runner | [https://just.systems/man/en/introduction.html](https://just.systems/man/en/introduction.html) |
| uv | python package and project manager | [https://docs.astral.sh/uv/getting-started/installation/](https://docs.astral.sh/uv/getting-started/installation/) |
| node | build / run configuration webpage via typescript, vite, and svelte | [https://nodejs.org/en/download](https://nodejs.org/en/download) |
| probe-rs | flash and debug embedded systems | [https://probe.rs/docs/getting-started/installation/](https://probe.rs/docs/getting-started/installation/) |
| wasm-pack | used for TS - rust interop | `cargo install wasm-pack` |
| md-book | documentation generator | [https://rust-lang.github.io/mdBook/guide/installation.html](https://rust-lang.github.io/mdBook/guide/installation.html) |


### Windows

The easiest way to develop on Windows is to install a Linux partition and dual boot. Check out of [one the many guides online](https://linuxblog.io/dual-boot-linux-windows-install-guide/).

You can develop without dual booting using [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install).
WSL2 cannot directly communicate with USB devices, so check out [this guide](https://learn.microsoft.com/en-us/windows/wsl/connect-usb) for a work around.
