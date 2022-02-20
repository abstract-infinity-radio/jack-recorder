# Building

* `cargo build` for local build
* for cross-compiling to linux:
    * build a docker image: `docker build -t jack_recorder_build_image .`
    * build a binary: `docker run --rm -ti -v $(pwd):/work jack_recorder_build_image bash -c 'cd /work ; cargo build'`
    * optionally copy to server: `scp target-linux/debug/jack_recorder kohai.ijs.si:/opt/jack_recorder`