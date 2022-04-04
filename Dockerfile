FROM rustembedded/cross:x86_64-unknown-linux-gnu-0.2.1
ENV DEBIAN_FRONTEND noninteractive
RUN apt update && apt install -y jackd2 libjack-jackd2-dev