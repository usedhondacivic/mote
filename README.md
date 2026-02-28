# Mote

[![Build and Test](https://github.com/mote-robotics/mote-core/actions/workflows/build-test.yaml/badge.svg)](https://github.com/mote-robotics/mote-core/actions/workflows/build-test.yaml)
[![Deploy Docs](https://github.com/mote-robotics/mote-core/actions/workflows/deploy.yaml/badge.svg)](https://github.com/mote-robotics/mote-core/actions/workflows/deploy.yaml)

A low cost and high confidence mobile robot.

## Motivation

Robotics is more fun with real hardware.

The ["low cost"](https://www.robotshop.com/products/clearpath-robotics-turtlebot-4-mobile-robot) robots currently on the market are prohibitively expensive for aspiring engineers.
Mote is my answer to the problem.

## History

Spiritual successor of [the Little Red Rover project](https://github.com/little-red-rover).

For the past two years, Little Red Rover has been used to teach the Foundations of Robotics course at Cornell University. Mote is the product of the many lessons learned during that effort.

## Project Structure

### Rust Packages
* mote-firmware :
    * Firmware for the RP2350 based Mote circuit board. Built with [Embassy](https://github.com/embassy-rs/embassy/).
* mote-api
    * Message definitions for communication to and from Mote.
    * SansIO communication driver, with bindings for Python and TypeScript
* mote-configuration
    * [Web app](https://mote-robotics.github.io/mote-core/configuration/) for configuration and debugging. Built with [Svelte](https://svelte.dev/) and [WebAssembly](https://wasm-bindgen.github.io/wasm-bindgen/).

### Hardware
* mote-hardware 
    * KiCAD board design files.

### Documentation
* web/mote-book
    * [Documentation and examples](https://mote-robotics.github.io/mote-core/), build with [mdBook](https://rust-lang.github.io/mdBook/).

