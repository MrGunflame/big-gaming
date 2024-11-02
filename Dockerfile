# Use an old version of Debian so our day does not
# get ruined by glibc again.
FROM debian:bullseye as builder

RUN apt-get update && apt-get install -y curl build-essential ca-certificates python3 python

RUN useradd -m -s /bin/bash game

RUN mkdir /game
RUN chown game /game
COPY . /game

USER game

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh -s -- --no-modify-path -y

WORKDIR /game
RUN /game/build.sh
