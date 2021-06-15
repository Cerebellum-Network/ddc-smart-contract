# DDC Smart Contract On Ink!

This smart contract takes deposit from DDC node wallet; then accepts data metrics reported by the paying node; then store and increment the metrics in a hashmap.

## Release Notes
#### vNext
* ...
#### v1.1.0
* Clean up the contract
* Added DDC Nodes API: add/list/delete
* Removed revoke_membership
* Removed token
#### v1.0.0
* DDC Smart Contract v1.0.0
    * Basic functionality added
* Derivative Assets Smart Contrat v0.1.0

## How to create Smart Contract Artificats

1. Clone this repository
1. Install build tools ([ink setup](https://substrate.dev/substrate-contracts-workshop/#/0/setup)):
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


## Deploy Smart Contract and test it
In order to deploy and test Smart Contract use [Quick Start Guide](https://github.com/Cerebellum-Network/private-standalone-network-node/blob/dev/docs/tutorial.md#quick-start-guide).

## Specification
See [Specification](./SPECIFICATION.md)

## Endowment

The endowment is the balance the deployer give to the contract upon deployment

Substrate's official recommendation is 1000 endowments - 1000 cere coins

Unlike Ethereum, the contract's balance seems to be this endowment

If someone pays 10 Cere subscription fee, the contract balance doesn't change, it is still 1000 endowment, not 1010;

It seems that when you refund a customer, say, paying back 10 Cere, this balance will be deducted from the endowment, now the endowment becomes 990

It seems the endowment is eroded (decreased) every block, see this official document

line #702 /// Query how many blocks the contract stays alive given that the amount endowment

https://github.com/paritytech/substrate/blob/master/frame/contracts/src/lib.rs

lacks official explanation, the only useful info found is this function

/// Query how many blocks the contract stays alive given that the amount endowment
/// and consumed storage does not change.
pub fn rent_projection(address: T::AccountId) -> RentProjectionResult<T::BlockNumber> {
	Rent::<T, PrefabWasmModule<T>>::compute_projection(&address)
}



