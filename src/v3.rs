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
        /// All the Buckets that are regulated by this contract.
        buckets: Stash<Bucket>,
        miners: HashMap<AccountId, Miner>,
        broker_miner_recommendations: HashMap<(AccountId, AccountId), u64>,
    }

    pub struct Bucket {
        owner_id: AccountId,
        committee: Committee,
        deposit: Balance,
    }

    pub struct Committee {
        /// A stable set of Brokers.
        brokers: HashMap<AccountId, Broker>,
        /// How many copies of the data or service are considered safe and sufficient for operations.
        target_miner_count: u8,
        /// The current Miners. Ideally, there should be target_replication_factor of them.
        miners: HashMap<AccountId, Miner>,
    }

    pub type BucketId = u32;

    pub struct Broker {}

    pub struct Miner {
        available_buckets: u64,
        rent: u64,
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
                committee: Committee {
                    brokers: HashMap::new(),
                    target_miner_count: 3,
                    miners: HashMap::new(),
                },
            };
            let bucket_id = self.buckets.put(bucket);
            Ok(bucket_id)
        }

        #[ink(message)]
        pub fn owner_topup(&mut self, bucket_id: BucketId) -> Result<()> {
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
        pub fn owner_withdraw(&mut self, bucket_id: BucketId) -> Result<()> {
            match self.buckets.get_mut(bucket_id) {
                None => Error,
                Some(bucket) => {
                    let value = bucket.deposit;
                    bucket.deposit = 0;
                    self.env().transfer(bucket.owner_id, value)?;
                    Ok(())
                }
            }
        }

        /// An Owner reports that he is using the service of a Miner, e.g. store data.
        /// The Owner requests that Brokers start verifying this service.
        #[ink(message)]
        pub fn owner_use_miner(&mut self, bucket_id: BucketId, miner_id: AccountId, data_checksum, data_size, miner_signature) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_owner(bucket)?;
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

        /// A Miner accepts to provide service to an Owner, e.g. store his data.
        /// The Miner becomes subject to verification by Brokers.
        #[ink(message)]
        pub fn miner_ack_usage(&mut self, bucket_id: BucketId, data_checksum, data_size) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_miner(bucket)?;
            Ok(())
        }

        /// Check if account is a current miner in a bucket.
        fn only_miner(&self, bucket: &Bucket) -> Result<AccountId> {
            let caller = self.env().caller();
            if bucket.committee.miners.contains_key(caller) {
                Ok(caller)
            } else {
                Err(Error)
            }
        }


        // ---- As Broker ----

        #[ink(message)]
        pub fn recommend_miner(&mut self, miner_id: AccountId, score: u64) -> Result<()> {
            let broker_id = self.env().caller();
            self.broker_miner_recommendations.insert((broker_id, miner_id), score);
            Ok(())
        }

        #[ink(message)]
        pub fn start_paying_miner(&mut self, bucket_id: BucketId, miner_id: AccountId, rent: u64) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_broker(bucket)?;
            // TODO: check that rent argument equals miner.rent (this prevents a race condition).
            // TODO: add miner if not exists, with start_at: now.
            // TODO: decrease miner’s available resources.
            // Note: using the rent from the miner makes him auto-agree on the deal.
            Ok(())
        }

        #[ink(message)]
        pub fn stop_paying_miner(&mut self, bucket_id: BucketId, miner_id: AccountId) -> Result<()> {
            let bucket = self.buckets.get_mut(bucket_id).ok_or(Error)?;
            self.only_broker(bucket)?;
            // TODO: pay out miner earnings from bucket deposit (similar to withdraw).
            // TODO: remove miner from the bucket.
            // TODO: increase miner’s available resources.
            Ok(())
        }

        /// Check if account is a current broker in a bucket.
        fn only_broker(&self, bucket: &Bucket) -> Result<AccountId> {
            let caller = self.env().caller();
            if bucket.committee.brokers.contains_key(caller) {
                Ok(caller)
            } else {
                Err(Error)
            }
        }
    }

    // ---- Utils ----
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}
