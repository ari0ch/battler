# Built by Render from the repository root (see render.yaml):
#   docker build -f crates/server/Dockerfile .

# Stage 1: compile. The official Rust image has rustup, so rust-toolchain.toml
# picks the toolchain just as it does locally and in CI.
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p battler-server

# Stage 2: a small runtime image with just the binary.
FROM debian:bookworm-slim
COPY --from=build /src/target/release/battler-server /usr/local/bin/battler-server
ENV PORT=10000
EXPOSE 10000
CMD ["battler-server"]
