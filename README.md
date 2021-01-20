# CERE01: A Standard For Real-world App Assets On Ink!

Derivative Asset support for the enterprise needs, with attributes such as expiration, limit on transfers, longitudinal unlocking, redemptions, etc.

This doc will explain:
* How to create Smart Contract artifacts
* How to start using it

## 1. How to create Smart Contract Artificats
1. Clone this repository
2. Change dir:
```bash
cd cere01
```
3. Now you can either test or build artifacts:

3.1 Test Smart Contract Source Code
```bash
cargo +nightly test
```

3.2 Build Smart Contract
```bash
cargo +nightly contract build
```

3.3 Generage Contract Metadata
```bash
cargo +nightly contract generate-metadata
```

## 2. Deploy Smart Contract and test it
In order to deploy and test Smart Contract use [Quick Start Guide](https://github.com/Cerebellum-Network/private-standalone-network-node/blob/dev/docs/tutorial.md#quick-start-guide).

## Specification
See [Specification](./cere01/specification.md)
