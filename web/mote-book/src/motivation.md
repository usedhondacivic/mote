# Motivation



## What?

Mote is an affordable and intuitive robot that makes it a joy to learn robotics.

## Why?

Robotics is more fun with real hardware.

Existing hardware solutions are prohibitively expensive for aspiring engineers and provide a frustrating development experience.
It doesn't have to be this way.

## How?

### Status Quo

Mobile robots (like Mote) have three core operations:
1. Collecting observations (reading sensor data)
2. Processing observations (computing odometry, localization, etc)
3. Responding to observations (generating actuator movement)

Traditionally, all three steps are run onboard the robot. 
This requires a real time processor (likely an microcontroller) for (1) and (3), paired with a single board computer (SBC) for (2).

The setup works alright, but relies on an expensive and power hungry SBC that tends to complicate system setup and encourage brittle and opaque configurations.
Can we do better?

### Mote's Approach

Mote forgoes an onboard SBC by wirelessly offloading data processing to an external base station.
The base station can be any computer, running any operating system, and is most often just the user's laptop.

Buy omiting the SBC Mote can get away with a miniscule power and computation budget, dropping the price and greating improving runtime.
Perfomance actually *improves* without the SBC, as even outdated laptops can outperform the average SBC.

## Where?
