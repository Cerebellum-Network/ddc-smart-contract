# CERE01: A Standard For Real-world App Assets On Ink!

Derivative Asset support for the enterprise needs, with attributes such as expiration, limit on transfers, longitudinal unlocking, redemptions, etc.

This doc will explain:
* How to create Smart Contract artifacts
* How to start using it

## How to create Smart Contract Artificats

1. Clone this repository
2. Install build tools ([ink setup](https://substrate.dev/substrate-contracts-workshop/#/0/setup)):
    ```bash
    rustup component add rust-src --toolchain nightly
    rustup target add wasm32-unknown-unknown --toolchain stable
    cargo install cargo-contract --vers ^0.12 --force --locked
    
    # Wasm tools (https://github.com/WebAssembly/binaryen/releases)
    # Ubuntu. Install default then upgrade to version >= 99.
    sudo apt install binaryen
    wget http://de.archive.ubuntu.com/ubuntu/pool/universe/b/binaryen/binaryen_99-3_amd64.deb
    sudo dpkg -i binaryen_99-3_amd64.deb
    # MacOS
    brew install binaryen
    ```
3. Change directory:
    ```bash
    cd cere01
    # or cd cere02
    ```
4. Now you can either test or build artifacts:
    * Test Smart Contract Source Code
    ```bash
    cargo +nightly test
    ```
    In case of any issues, try to specify version:
    ```bash
    cargo +nightly-2020-10-06 test
    ```
    * Build Smart Contract
    ```bash
    cargo +nightly contract build
    ```
    * Upload `ddc.wasm` and `metadata.json` using a block viewer (like [Cere Testnet](https://block-viewer.cere.network/?rpc=wss%3A%2F%2Frpc.testnet.cere.network%3A9945#/contracts))

## Deploy Smart Contract and test it
In order to deploy and test Smart Contract use [Quick Start Guide](https://github.com/Cerebellum-Network/private-standalone-network-node/blob/dev/docs/tutorial.md#quick-start-guide).

## Specification
See [Specification](./cere01/specification.md)
