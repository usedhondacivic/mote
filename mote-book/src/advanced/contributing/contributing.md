# Contributing

Mote is an open-source project hosted by Cornell University's [EmPRISE Lab](https://github.com/empriselab). Thoughful contributions are welcome and appreciated.

First time contributors are recommended to read through the documentation, then check out the [open issues](https://github.com/empriselab/mote-core/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22).

## Project Structure

`mote-core` contains the core libraries required for mote to function. It is composed of the following components:

- `mote-firmware`
    - Embedded firmware for the RP2350 MCU
- `mote-api`
    - Provides Rust, Python, and Typescript libraries for sending and receiving messages from Mote
- `mote-configuration`
    - Webpage used to configure / debug Mote
    - Uses the `mote-api` Typescript library to read / write configuration values, connect the robot to the network, and display errors
- `mote-hardware`
    - KiCAD circuit board design files
- `mote-book`
    - You're reading it!
    - Documentation and tutorials

Extension repositories use libraries from `mote-api` and implement bridges to other frameworks.
`mote-ros`, for example, wraps `mote-api`'s python communication library to implement a Robot Operating System (ROS) package for the robot.
