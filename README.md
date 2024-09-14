# pico-tflmicro-sys
Rust bindings of TensorFlow Lite for Microcontrollers for Raspberry Pi Pico.

# IMPORTANT NOTICE!
This is nonfunctional, instead, it is recommended to use [burn](https://github.com/tracel-ai/burn/). Follow the burn guide for the Raspberry Pi Pico [here](https://burn.dev/burn-book/advanced/no-std.html). See the full source code example [here](https://github.com/tracel-ai/burn/tree/main/examples/raspberry-pi-pico)

## Why?
[pico-tflmicro](https://github.com/raspberrypi/pico-tflmicro) is a port the Raspberry Pi company created to run TensorFlow on their microcontrollers. To run TensorFlow natively on the pico with the best support and the fastest, it seemed best to create bindings for this project.
`pico-tflmicro-sys` is intended to be bare bones and built on top of in order to provide the functionality you wish to have.

## Installation
```
cargo add pico-tflmicro-sys
```
or
```
[dependencies]
pico-tflmicro-sys = { version="0.1.0" }
```

### Features
Add the `build` feature if you want to build from scratch, otherwise this uses a prebuilt binary.
