# pico-tflmicro-sys
Rust bindings of TensorFlow Lite for Microcontrollers for Raspberry Pi Pico.

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
