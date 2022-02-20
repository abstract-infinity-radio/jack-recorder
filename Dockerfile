FROM rust:1.58-slim

ENV CARGO_TARGET_DIR=target-linux
ENV DEBIAN_FRONTEND=noninteractive

RUN apt update && apt install -y jackd2 libjack-jackd2-dev