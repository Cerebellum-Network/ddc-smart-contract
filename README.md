# CERE01: A Standard For Real-world App Assets On Ink!

Derivative Asset support for the enterprise needs, with attributes such as expiration, limit on transfers, longitudinal unlocking, redemptions, etc.

## Specification
See [Specification](./cere01/specification.md)

## How to create Smart Contract Artificats

### Test Smart Contract Source Code
```bash
cargo +nightly test
```

### Build Smart Contract
```bash
cargo +nightly contract build
```

### Generage Contract Metadata
```bash
cargo +nightly contract generate-metadata
```

