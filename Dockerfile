ARG RUST_VERSION=1.52.1
FROM rust:$RUST_VERSION

WORKDIR /smart-contracts/cere01
COPY /cere01 /smart-contracts/cere01
COPY /cere02 /smart-contracts/cere02

# Update new packages
RUN apt update

# Install all dependencies
ARG CARGO_CONTRACT_VERSION=0.12.1
RUN rustup default stable
RUN rustup update
RUN rustup update nightly
RUN rustup component add rust-src --toolchain nightly
RUN rustup target add wasm32-unknown-unknown --toolchain stable
RUN cargo install cargo-contract --vers ^$CARGO_CONTRACT_VERSION --force --locked
RUN apt install -y binaryen
RUN rustup show

# Run tests and build
RUN cargo +nightly test
RUN cargo +nightly contract build
