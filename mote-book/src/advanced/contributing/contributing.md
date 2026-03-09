# Contributing

Mote is an open-source project hosted by Cornell University's [EmPRISE Lab](https://github.com/empriselab). Thoughful contributions are welcome and appreciated.

First time contributors are recommended to read through the documentation, then check out the [open issues](https://github.com/empriselab/mote-core/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22).

## Project Structure

`mote-core` contains the core libraries required for Mote to function. It includes the following components:

- `mote-firmware`
    - Embedded firmware for the RP2350 MCU
- `mote-api`
    - Defines message types and serialization protocols for communicating with Mote
- `mote-ffi`
    - Foreign Function Interface (FFI)
    - Wraps `mote-api` in Python, C++, and Typescript libraries, allowing popular application languages to communicate with Mote
- `mote-configuration`
    - Webpage used to configure / debug Mote
    - Uses the `mote-ffi` Typescript library to read / write configuration values, connect the robot to the network, and display errors
- `mote-hardware`
    - KiCAD circuit board design files
- `mote-book`
    - You're reading it!
    - Documentation and tutorials

Extension repositories use libraries from `mote-ffi` to implement bridges to other frameworks.
[`mote-ros`](https://github.com/empriselab/mote-ros), for example, wraps `mote-ffi`'s C++ library to implement a Robot Operating System (ROS) node for the robot.
