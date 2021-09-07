# ===== FIRST STAGE ======
FROM rust:1.54 as builder

RUN apt-get update && \
    apt-get -y upgrade

WORKDIR /ddc-smart-contract
COPY . /ddc-smart-contract

# Install binaryen
RUN curl --silent https://api.github.com/repos/WebAssembly/binaryen/releases/latest | \
		egrep --only-matching 'https://github.com/WebAssembly/binaryen/releases/download/version_[0-9]+/binaryen-version_[0-9]+-x86_64-linux.tar.gz' | \
		head -n1 | \
		xargs curl -L -O && \
	tar -xvzf binaryen-version_*-x86_64-linux.tar.gz  && \
	rm binaryen-version_*-x86_64-linux.tar.gz && \
	chmod +x binaryen-version_*/bin/wasm-opt && \
	mv binaryen-version_*/bin/wasm-opt /usr/local/bin/ && \
	rm -rf binaryen-version_*/

# Install cargo-contract
RUN rustup toolchain install nightly-2021-09-06 && \
	rustup default nightly-2021-09-06 && \
	rustup component add rust-src --toolchain nightly-2021-09-06 && \
	rustup target add wasm32-unknown-unknown --toolchain nightly-2021-09-06 && \
	cargo install cargo-contract --version 0.14.0 --force --locked

# Run tests
RUN cargo test

# Build contract
RUN cargo contract build

# ===== SECOND STAGE ======
FROM phusion/baseimage:0.11
WORKDIR /ddc-smart-contract
COPY --from=builder /ddc-smart-contract/target/ink/ddc.contract /ddc-smart-contract/artifacts/
COPY --from=builder /ddc-smart-contract/target/ink/ddc.wasm /ddc-smart-contract/artifacts/
COPY --from=builder /ddc-smart-contract/target/ink/metadata.json /ddc-smart-contract/artifacts/
