# Mote

[![Rust Build and Test](https://github.com/usedhondacivic/mote/actions/workflows/rust.yml/badge.svg)](https://github.com/usedhondacivic/mote/actions/workflows/rust.yml)
[![Deploy Docs](https://github.com/usedhondacivic/mote/actions/workflows/deploy.yaml/badge.svg)](https://github.com/usedhondacivic/mote/actions/workflows/deploy.yaml)

A low cost, high confidence mobile robot.

## Motivation

Robotics is more fun with real hardware.

Today's ["low cost"](https://www.robotshop.com/products/clearpath-robotics-turtlebot-4-mobile-robot) robots are prohibitively expensive for many aspiring engineers.
Mote is my answer to that problem.

## History

Mote is the spiritual successor of my master's thesis, [the Little Red Rover project](https://github.com/little-red-rover).

I used the Little Red Rover platform to teach a semester long course on the foundations of robotics.
Mote is the product of the many lessons I learned during that effort.

## Project Structure

### Rust Packages
* mote-firmware :
    * Firmware for the RP2350 based Mote circuit board. Built with [Embassy](https://github.com/embassy-rs/embassy).
* mote-messages
    * Message definitions for communication to and from Mote.
* mote-sansio-driver
    * SansIO communication driver.
* web/mote-configuration
    * [Web app](https://michael-crum.com/mote/configuration/) for configuration and debugging.

### Hardware
* mote-hardware 
    * KiCAD board design files.

### Documentation
* web/mote-book
    * [Documentation and examples](https://michael-crum.com/mote/), build with [mdBook](https://rust-lang.github.io/mdBook/).

