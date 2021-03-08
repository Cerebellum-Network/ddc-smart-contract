# Purpose of the smart contract

This smart contract takes deposit from DDC node wallet; then accepts data metrics reported by the paying node; then store and increment the metrics in a hashmap.

# Data structure of hashmap

Key: AccountId
Value: Vec![partition_id, data_received, data_replicated, request_received, request_replicated]

# Initialize the contract

Initialize the contract with 1) miminum deposit required; 2) name of this contract - a string;

# Test

rustup default nightly
cargo test

Tested fully work with local blockchain

# Build

Install all dependencies for Rust/Ink!;

Install Rust Ink! Contract with binaryen wasm-opt;

cargo contract build

# Deploy 

Deploy the ddc.contract artifacts to local blockchain or testnet;

# TODO

A lot to do to make it a full blown smart contract rich business logic;