# Mote

[![Rust Build and Test](https://github.com/usedhondacivic/mote/actions/workflows/rust.yml/badge.svg)](https://github.com/usedhondacivic/mote/actions/workflows/rust.yml)
[![Deploy Docs](https://github.com/usedhondacivic/mote/actions/workflows/deploy.yaml/badge.svg)](https://github.com/usedhondacivic/mote/actions/workflows/deploy.yaml)

A low cost, high confidence mobile robot.

## Motivation

Learning robotics is better with access to real hardware, but existing "[low cost](https://www.robotshop.com/products/clearpath-robotics-turtlebot-4-mobile-robot)" robots are prohibitively expensive for many aspiring engineers.
Mote is an (actually) low cost mobile robotics platform built to teach the next generation of engineers.
Leveraging robust design, high confidence tooling, and extensive documentation to provide a first class experience for students and hobbyists alike, Mote lowers the barrier of entry for research level robotics. 
## History

Mote is the spiritual successor of my master's thesis, [the Little Red Rover project](https://github.com/little-red-rover).
During my thesis, I lead a class of 15 students through a semester of excercises using the Little Red Rover platform.
Mote is the product of the many lessons I learned during that effort.

## Project Structure

### Rust Packages
* mote-firmware
    * Rust firmware for the RP2350 based Mote circuit board. Built with [Embassy](https://github.com/embassy-rs/embassy).
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

