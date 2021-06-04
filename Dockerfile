# ===== FIRST STAGE ======
ARG RUST_VERSION=1.52.1
FROM rust:$RUST_VERSION as builder

RUN apt-get -y update && \
    apt-get -y upgrade && \
    apt-get install -y binaryen wget

WORKDIR /smart-contracts
COPY ./cere01 /smart-contracts/cere01
COPY ./cere02 /smart-contracts/cere02

# Install all dependencies
ARG CARGO_CONTRACT_VERSION=0.12.1
RUN rustup default stable && \
	rustup update && \
	rustup update nightly && \
	rustup component add rust-src --toolchain nightly && \
	rustup target add wasm32-unknown-unknown --toolchain stable && \
	cargo install cargo-contract --vers ^$CARGO_CONTRACT_VERSION --force --locked
RUN	wget http://ftp.us.debian.org/debian/pool/main/libx/libxcrypt/libcrypt1_4.4.18-4_amd64.deb && \
	dpkg -i libcrypt1_4.4.18-4_amd64.deb && \
	wget http://ftp.us.debian.org/debian/pool/main/g/gcc-10/gcc-10-base_10.2.1-6_amd64.deb && \
	dpkg -i gcc-10-base_10.2.1-6_amd64.deb && \
	wget http://ftp.us.debian.org/debian/pool/main/g/gcc-10/libgcc-s1_10.2.1-6_amd64.deb && \
	dpkg -i libgcc-s1_10.2.1-6_amd64.deb && \
	wget http://ftp.us.debian.org/debian/pool/main/g/glibc/libc6_2.31-12_amd64.deb && \
	dpkg -i libc6_2.31-12_amd64.deb && \
	wget http://ftp.us.debian.org/debian/pool/main/g/gcc-10/libstdc++6_10.2.1-6_amd64.deb && \
    dpkg -i libstdc++6_10.2.1-6_amd64.deb && \
	wget http://de.archive.ubuntu.com/ubuntu/pool/universe/b/binaryen/binaryen_99-3_amd64.deb && \
	dpkg -i binaryen_99-3_amd64.deb

# Run tests and build
WORKDIR /smart-contracts/cere02
RUN cargo +nightly test && \
	cargo +nightly contract build

# ===== SECOND STAGE ======
FROM phusion/baseimage:0.11
WORKDIR /smart-contracts
COPY --from=builder /smart-contracts/cere02/target/ink/ddc.wasm /smart-contracts/artifacts
COPY --from=builder /smart-contracts/cere02/target/ink/metadata.json /smart-contracts/artifacts
