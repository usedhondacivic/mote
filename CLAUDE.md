# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Mote is a low-cost, open-source mobile robot platform for education (Cornell EmPRISE Lab). The monorepo contains firmware, protocol definitions, FFI bindings, a web configurator, and documentation.

## Commands

The project uses `just` as the task runner. Run `just` from the repo root or from within any package directory.

### Root
```bash
just ci              # Full CI suite (firmware + api + book + config)
just ci-web-artifact # Build web artifacts for GitHub Pages
```

### mote-firmware (Rust, RP2350)
```bash
cd mote-firmware
just build           # Compile firmware
just test            # Run unit tests (cargo test)
just lint            # clippy
just format-check    # rustfmt --check
just deploy          # Flash to device via probe-rs
just ci              # lint + format-check + build + test
```

### mote-api (Rust)
```bash
cd mote-api
just lint
just format-check
just ci
```

### mote-ffi (Rust + Python + WASM)
```bash
cd mote-ffi
just dev-setup       # Install Python deps (uv sync)
just prod-setup      # Build and install Python wheel
just build-wasm      # wasm-pack build
just test            # cargo test + Python type check (uv run ty check)
just lint
just format-check
just ci
```

### mote-configuration (Svelte/TypeScript)
```bash
cd mote-configuration
npm run dev          # Dev server (Vite)
npm run build        # Production build
npm run check        # Svelte type checking
just run-dev         # Alias for npm run dev
just ci              # Type check + build
```

### mote-book (mdBook)
```bash
cd mote-book
just build           # Build docs
just open            # Build and open in browser
just test            # Test code examples
just ci
```

## Architecture

### Data Flow
```
Host/Python app ──► mote-api (MoteComms) ──► UDP/WiFi ──► mote-firmware
Web browser     ──► mote-ffi (WASM)      ──► USB Serial ──► mote-firmware
```

### Package Roles

**mote-api**: Transport-agnostic (SansIO) message protocol. `MoteComms<MTU, I, O>` encodes/decodes messages using Bitcode + COBS framing. Provides type aliases:
- `MoteLink` / `HostLink` — runtime UDP (MTU=1400)
- `MoteConfigLink` / `HostConfigLink` — USB serial (MTU=64)

Message definitions live in `src/messages/host_to_mote.rs` and `src/messages/mote_to_host.rs`.

**mote-firmware**: Embassy async runtime on RP2350. Dual-core split:
- Core 0: WiFi (CYW43), mDNS, TCP/UDP servers
- Core 1: USB serial, LiDAR (RP LiDAR C1), drive base (motors + PID + encoders), power gate

Peripherals are allocated statically via `assign_resources!` in `src/tasks.rs`. Global config state is a mutex (`CONFIGURATION_STATE`). LiDAR scan offloading uses an Embassy channel (`MOTE_TO_HOST_DATA_OFFLOAD`).

**mote-ffi**: Wraps mote-api for external languages. `MoteCommsFFI` adds JSON serialization shim on top of `MoteComms`.
- `python_ffi` feature → PyO3 bindings (`mote_link.mote_ffi` Python module)
- `wasm_ffi` feature → wasm-bindgen bindings (used by mote-configuration)

**mote-configuration**: Svelte 5 app. Uses Web Serial API for USB connection, imports mote-ffi WASM for message handling. Components: `Identification`, `Networks`, `Diagnostics`.

**mote-book**: mdBook documentation at `mote-book/src/`. Contributing guides (build/test/deploy details) are in `advanced/contributing/`.

**mote-hardware**: KiCAD PCB files only.

## Key Conventions

- Rust toolchain is pinned via `mote-firmware/rust-toolchain.toml` (needed for Embassy + embedded target)
- Python tooling uses `uv` (not pip/poetry)
- The FFI layer's `Link` struct (both Python and WASM) exposes: `send()`, `poll_transmit()`, `handle_receive()`, `poll_receive()`
- Power gating: USB detection (500mA → 1.5A → 3A thresholds) controls WiFi/drive base startup sequence
