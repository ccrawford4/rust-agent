FROM rust:1.92 as builder


WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN cargo build --release

COPY ./src ./src
