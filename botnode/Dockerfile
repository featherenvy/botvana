# Leveraging the pre-built Docker images with
# cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1.58.0 AS chef
WORKDIR app
RUN apt-get update && apt-get -y install cmake

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/.cargo .cargo
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin botnode

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye-slim AS runtime
RUN apt-get update && apt-get -y install libatomic1 libcurl4
WORKDIR app
COPY --from=builder /app/target/release/botnode /usr/local/bin
ENTRYPOINT ["/usr/local/bin/botnode"]
