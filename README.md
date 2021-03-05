# CERE01: A Standard For Real-world App Assets On Ink!

Derivative Asset support for the enterprise needs, with attributes such as expiration, limit on transfers, longitudinal unlocking, redemptions, etc.

This doc will explain:
* How to create Smart Contract artifacts
* How to start using it

## How to create Smart Contract Artificats

1. Clone this repository
1. Change directory:
    ```bash
    cd cere01
    ```
1. Now you can either test or build artifacts:
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
    * Generage Contract Metadata
    ```bash
    cargo +nightly contract generate-metadata
    ```

## Deploy Smart Contract and test it
In order to deploy and test Smart Contract use [Quick Start Guide](https://github.com/Cerebellum-Network/private-standalone-network-node/blob/dev/docs/tutorial.md#quick-start-guide).

## Specification
See [Specification](./cere01/specification.md)
