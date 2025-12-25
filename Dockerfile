FROM rust:1.92 AS builder

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /usr/src/app/target/release/rust-agent /usr/local/bin/rust-agent

CMD ["rust-agent"]
