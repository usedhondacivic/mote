# Building, Testing, and Running

## Setup

Follow the instructions in [Development Environment](advanced/contributing/dev_env.html) to setup the required tools.

Make a clone of the repository: 
```bash
git clone git@github.com:empriselab/mote-core.git
```
or 
```bash
git clone https://github.com/empriselab/mote-core.git
```

## Build

Compile source and generate executable artifacts.

### `mote-firmware`

```bash
cd mote-firmware
just build
```

### `mote-configuration`

```bash
cd mote-configuration
just build
```

### `mote-book`

```bash
cd mote-book
just build
```

## Test

Run unit tests.

### `mote-firmware`

```bash
cd mote-firmware
just test
```

### `mote-book`

```bash
cd mote-book
just test
```

## Run

Run a deployment target.

### `mote-firmware`

Connect to Mote using [a SWD debug probe](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html). Then,

```bash
cd mote-firmware
just deploy
```

### `mote-configuration`

```bash
cd mote-configuration
just run-dev
```

Click the link provided by Vite, and the configuration page will open in your browser.

### `mote-book`

```bash
cd mote-book
just open
```

## Deploy

Release / upload build artifacts for public consumption.

### `mote-firmware`

Coming soon in [#11](https://github.com/empriselab/mote-core/issues/11).

### `mote-book`, `mote-configuration`

Web based targets are deployed automatically to GitHub pages using the [Deploy to Pages workflow](https://github.com/empriselab/mote-core/blob/main/.github/workflows/deploy.yaml).

### `mote-ffi`

Coming soon in [#17](https://github.com/empriselab/mote-core/issues/17)
