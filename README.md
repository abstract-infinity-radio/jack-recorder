# Building

* for local build: `cargo build`
* for cross-compiling:
    * build an appropriate image with `docker build -t rustembedded/cross:withjack-0.2.1 .`
    * initiate a cross platform build with `cross build --target x86_64-unknown-linux-gnu`
* more info: https://github.com/cross-rs/cross
* optionally copy to server:
  * `scp target/x86_64-unknown-linux-gnu/debug/jack_recorder kohai.ijs.si:/opt/jack_recorder/`
  * `scp target/x86_64-unknown-linux-gnu/debug/jack_recorder_server kohai.ijs.si:/opt/jack_recorder/`

# Running

## API Server

* `jack_recorder_server -p 8080 -o /path/to/output/directory`
## Standalone CLI

* `jack_recorder listports` to list the available ports
* `jack_recorder record -o /path/to/output/directory` to record all avaialable inputs and store recordings in the output directory
* use `-v` flag to output recording queue length periodically
