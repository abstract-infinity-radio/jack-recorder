# Building

## Local build

Use `cargo build` to build to project.

## Cross compilation for x64 linux

* build an appropriate image with `docker build -t rustembedded/cross:withjack-0.2.1 .`
* initiate a build with `cross build --target x86_64-unknown-linux-gnu`
* copy binaries from `target/x86_64-unknown-linux-gnu`
* more info: https://github.com/cross-rs/cross

## Cross compilation using a custom docker image

* use a simple rust docker image with Jack dependencies. A sample `Dockerfile`:

  ```dockerfile
  FROM rust:1.58-slim

  ENV CARGO_TARGET_DIR=target-linux
  ENV DEBIAN_FRONTEND=noninteractive

  RUN apt update && apt install -y jackd2 libjack-jackd2-dev  
  ```

* build a docker image with `docker build -t jack_recorder_builder .`
* initiate a build with:
  ```sh
  docker run --rm -it \
      -e CARGO_TARGET_DIR=/target \
      -v ~/.cargo:/cargo \
      -v $(pwd):/project:ro -v $(pwd)/target/jack_recorder_builder:/target \
      -w /project \
      jack_recorder_builder \
      cargo build
  ```
* copy binaries from `target/jack_recorder_builder`

# Running

## API Server

Use `jack_recorder_server` to run the server. Port and output directory are mandatory.

```
USAGE:
    jack_recorder_server [OPTIONS] -o <OUTPUT_DIR> -p <PORT>

OPTIONS:
    -h, --help             Print help information
    -o <OUTPUT_DIR>        output directory
    -p <PORT>
    -v                     verbose output
    -V, --version          Print version information
```

* `jack_recorder_server -p 8080 -o /path/to/output/directory`

## Standalone CLI

```
JACK audio recorder

USAGE:
    jack_recorder [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -v               verbose output
    -V, --version    Print version information

SUBCOMMANDS:
    help      Print this message or the help of the given subcommand(s)
    list      List available JACK ports
    record    Record audio
```
