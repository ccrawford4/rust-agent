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
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-gnu ;; \
      arm64) echo aarch64-unknown-linux-gnu ;; \
      *) echo "Unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac > /rust_target && \
    rustup target add $(cat /rust_target)

COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --target $(cat /rust_target) --recipe-path recipe.json

# Build the source code
COPY . .
RUN cargo build --release --target $(cat /rust_target) && \
    cp target/$(cat /rust_target)/release/rust-agent /usr/local/bin/rust-agent

# Use a slimmer image and install runtime dependencies
FROM rust:1.92-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary and execute
COPY --from=builder /usr/local/bin/rust-agent /usr/local/bin/rust-agent
CMD ["rust-agent"]
