# Purpose of the smart contract

This smart contract takes deposit from DDC node wallet; then accepts data metrics reported by the paying node; then store and increment the metrics in a hashmap.


# Data structure of hashmap

self.balances is a HashMap
Key: AccountId
Value: Vec![tier_id, data_received, data_replicated, request_received, request_replicated]

self.service is a HashMap
Key: tier_id, 1,2,3 
Value: Vec![tier_id, tier_fee, tier_throughput_limit, tier_storage_limit]

# Initialize contract

Initialize the contract with these params

tier 3 fee,  // supposed to be the lowest
tier 3 throughput limit,
tier 3 storage limit,
tier 2 fee,  // medium level
tier 2 throughput limit,
tier 2 storage limit,
tier 1 fee, // most expensive level
tier 1 throughtput limit,
tier 1 storage limit,
name of the contract - a string such as "DDC"


# Install toolchains and dependencies

First, install rust developer environment according to this link

https://substrate.dev/docs/en/knowledgebase/getting-started/


Second, install Substrate Prerequisite and Canvas Node (a local test blockchain node)  

Do not follow everything! Only follow the steps till (including the Canvas Node)

https://substrate.dev/substrate-contracts-workshop/#/0/setup


Third, install ink! CLI, do not follow the instruction from the above page, instead

Go to ink! contract's Github page and follow the instruction here:

https://github.com/paritytech/cargo-contract

On a Ubuntu machine, you can simply run this command:

cargo install --force --features binaryen-as-dependency cargo-contract

The key is to install binaryen --version > 99 (99 or 100) in the system

The above command will force to install the newest version of binaryen

Mac OS should be a similar process

On a Windows machine, you have to manually compile binaryen and add it to your path


# Test

This is the working version rustup nightly
Normally it works under any rustup nightly for test

rustc 1.53.0-nightly (07e0e2ec2 2021-03-24)


rustup default nightly
cargo test

Tested fully work with local blockchain

# Build

cargo contract build

The 3 files should be generate in /target/ink folder: 

ddc.contract
ddc.wasm
metadata.json

# Deploy 

First, run the local test node per the instructions inside:

https://substrate.dev/substrate-contracts-workshop/#/0/running-a-substrate-node

Second, follow the instructions below to deploy the contract

https://substrate.dev/substrate-contracts-workshop/#/0/deploying-your-contract

Third, follow the instructions below to call your contract

https://substrate.dev/substrate-contracts-workshop/#/0/calling-your-contract


# Endowment

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





