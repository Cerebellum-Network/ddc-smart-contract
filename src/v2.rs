#![cfg_attr(not(feature = "std"), no_std)]
#![feature(proc_macro_hygiene)] // for tests in a separate file

use ink_lang as ink;

#[ink::contract]
mod ddc {
    use ink_env::AccountId;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::{
        collections::HashMap,
        collections::Stash,
        lazy::Lazy,
        lazy::LazyHashMap,
        traits::{PackedLayout, SpreadLayout},
    };
    use scale::{Decode, Encode};

    // ---- Storage ----
    #[ink(storage)]
    pub struct Ddc {
        buckets: Stash<Bucket>,
        miners: HashMap<AccountId, Miner>,
        broker_miner_recommendations: HashMap<(AccountId, AccountId), u64>,
    }

    pub struct Bucket {
        owner_id: AccountId,
        deposit: u64,
        miners: HashMap<AccountId, BucketMiner>,
    }

    pub struct BucketMiner {
        rent: u64,
        start_at: u64,
    }

    pub type BucketId = u32;

    pub struct Miner {
        available_buckets: u64,
        rent_per_bucket: u64,
        rent_per_size: u64,
        // total rent = rent_per_bucket + size * rent_per_size
    }


    impl Ddc {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                buckets: Stash::new(),
                miners: HashMap::new(),
                broker_miner_recommendations: HashMap::new(),
            }
        }


        // ---- As Owner ----

        #[ink(message)]
        pub fn create_bucket(&mut self) -> Result<BucketId> {
            let owner_id = self.env().caller();
            let bucket = Bucket {
                owner_id,
                deposit: 0,
                miners: HashMap::new(),
            };
            let bucket_id = self.buckets.put(bucket);
            Ok(bucket_id)
        }

        #[ink(message)]
        pub fn topup_bucket(&mut self, bucket_id: BucketId) -> Result<()> {
            match self.buckets.get_mut(bucket_id) {
                None => Error,
                Some(bucket) => {
                    let value = self.env().transferred_balance();
                    bucket.deposit += value;
                    Ok(())
                }
            }
        }

        #[ink(message)]
        pub fn start_paying_miner(&mut self, bucket_id: BucketId, miner_id: AccountId, rent: u64) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_owner(bucket)?;
            // TODO: check that rent argument equals miner.rent (this prevents a race condition).
            // TODO: add miner if not exists, with start_at: now.
            // TODO: decrease miner’s available resources.
            // Note: using the rent from the miner makes him auto-agree on the deal.
            Ok(())
        }

        #[ink(message)]
        pub fn stop_paying_miner(&mut self, bucket_id: BucketId, miner_id: AccountId) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_owner(bucket)?;
            // TODO: pay out miner earnings from bucket deposit (similar to withdraw).
            // TODO: remove miner from the bucket.
            // TODO: increase miner’s available resources.
            Ok(())
        }

        /// Check if account has permission to manage the bucket including handling money.
        fn only_owner(&self, bucket: &Bucket) -> Result<AccountId> {
            let caller = self.env().caller();
            if caller == bucket.owner_id {
                Ok(caller)
            } else {
                Err(Error)
            }
        }


        // ---- As Miner ----

        #[ink(message)]
        pub fn set_miner_info(&mut self, available_buckets: u64, rent: u64) -> Result<()> {
            let miner_id = self.env().caller();
            self.miners.insert(miner_id, Miner {
                available_buckets,
                rent,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn is_miner_paid(&self, bucket_id: BucketId, miner_id: AccountId) -> Result<bool> {
            // TODO: check that miner exists in self.buckets[bucket_id].miners[miner_id]
            Ok(false)
        }

        #[ink(message)]
        pub fn withdraw_miner_earnings(&mut self, bucket_id: BucketId) -> Result<()> {
            let miner_id = self.env().caller();
            // TODO: calculate earnings based on rent and start_time recorded in the bucket.
            // TODO: reset start_at.
            // TODO: deduct from deposit.
            // TODO: pay out to the miner.
            Ok(())
        }


        // ---- As Broker ----

        #[ink(message)]
        pub fn recommend_miner(&mut self, miner_id: AccountId, score: u64) -> Result<()> {
            let broker_id = self.env().caller();
            self.broker_miner_recommendations.insert((broker_id, miner_id), score);
            Ok(())
        }
    }

    // ---- Utils ----
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}
