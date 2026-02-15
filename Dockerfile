# Use Rust Official Image
FROM rust:1.92 AS chef

# Install cargo-chef for depency caching
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/app

# Create a planner stage
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build and cache depndencies
FROM chef AS builder
COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the source code
COPY . .
RUN cargo build --release

# Use a slimmer image and install runtime dependencies
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary and execute
COPY --from=builder /usr/src/app/target/release/rust-agent /usr/local/bin/rust-agent
CMD ["rust-agent"]
