# Eureka

Eureka is a protocol specification and research implementation repository.

The scope of eureka is to build accountability for a network of decentralised search nodes. The problem is split into four subproblems: extraction, transformation, load and query (ETLQ).

This repository is only intended for team learning and prototyping. The focus is on Plonky2 and risc-0.

## Building

Plonky2 requires nightly toolchain of the rust compiler at this time. If this is not configured automatically (see `rust-toolchain`) then to set toolchain to nighly, you can run
```
rustup override set nightly
```
in the root directory of the repository. 