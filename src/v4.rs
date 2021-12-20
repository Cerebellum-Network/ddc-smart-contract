#![cfg_attr(not(feature = "std"), no_std)]
#![feature(proc_macro_hygiene)] // for tests in a separate file

use ink_lang as ink;

#[ink::contract]
mod ddc {
    use ink_env::AccountId;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        collections::Stash,
        lazy::Lazy,
        traits::{PackedLayout, SpreadLayout},
    };
    use scale::{Decode, Encode};

    // ---- Storage ----
    pub struct Miner {
        available_buckets: u64,
        rent: u64,
    }

    pub struct Broker {
        recommended_miners: Stash<AccountId>,
    }

    pub struct BucketMiner {
        rent: u64,
        since: u64,
    }

    pub struct Bucket {
        owner: AccountId,
        deposit: u64,
        miners: StorageHashMap<AccountId, BucketMiner>,
    }

    #[ink(storage)]
    pub struct Ddc {
        miners: StorageHashMap<AccountId, Miner>,
        brokers: StorageHashMap<AccountId, Broker>,
        buckets: Stash<Bucket>,
    }

    impl Ddc {
        /// Constructor that initializes the contract
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                miners: StorageHashMap::new(),
                brokers: StorageHashMap::new(),
                buckets: Stash::new(),
            }
        }

        // ---- Anyone ----

        #[ink(message)]
        pub fn deposit(&mut self) -> Result<()> {
            Ok(())
        }


        // ---- As Consumer ----

        #[ink(message)]
        pub fn add_owner(&mut self, new_owner: AccountId) -> Result<()> {
            self.only_owner()?;
            Ok(())
        }

        #[ink(message)]
        pub fn permit_to_write(&mut self, new_writer: AccountId) -> Result<()> {
            self.only_writer()?;
            Ok(())
        }

        #[ink(message)]
        pub fn set_max_pay_rate(&mut self, rate: u64) -> Result<()> {
            self.only_owner()?;
            Ok(())
        }

        // ---- As Consumer or Provider ----
        #[ink(message)]
        pub fn trust_referee(&mut self, referee: AccountId) -> Result<()> {
            Ok(())
        }

        #[ink(message)]
        pub fn distrust_referee(&mut self, referee: AccountId) -> Result<()> {
            Ok(())
        }


        // ---- As Consumer or Referee ----
        #[ink(message)]
        pub fn release_payment(&mut self, provider: AccountId, amount: u64) -> Result<()> {
            self.only_referee_or_owner()?;
            Ok(())
        }

        #[ink(message)]
        pub fn trust_provider(&mut self, provider: AccountId) -> Result<()> {
            self.only_referee_or_owner()?;
            Ok(())
        }

        #[ink(message)]
        pub fn distrust_provider(&mut self, provider: AccountId) -> Result<()> {
            self.only_referee()?;
            Ok(())
        }


        // ---- Permissions ----

        /// Check if account has permission to manage the bucket including handling money.
        fn only_owner(&self) -> Result<AccountId> {
            let caller = self.env().caller();
            Ok(caller)
        }

        /// Check if account has permission to request storage.
        fn only_writer(&self) -> Result<AccountId> {
            let caller = self.env().caller();
            Ok(caller)
        }

        /// Check if account has permission to release payments or slash providers.
        fn only_referee(&self) -> Result<AccountId> {
            let caller = self.env().caller();
            Ok(caller)
        }

        /// Check if account has permission to release payments.
        fn only_referee_or_owner(&self) -> Result<AccountId> {
            let caller = self.env().caller();
            Ok(caller)
        }


        // ---- As Provider ----
        #[ink(message)]
        pub fn request_payment(&mut self, amount: u64) -> Result<()> {
            Ok(())
        }


        // ======== Proof-of-storage ========

        // ---- As consumer ----
        #[ink(message)]
        pub fn request_storage(&mut self, new_state: u256) -> Result<()> {
            self.only_writer()?;
            Ok(())
        }

        // ---- As provider ----
        #[ink(message)]
        pub fn stake(&mut self) -> Result<()> {
            Ok(())
        }

        #[ink(message)]
        pub fn ack_storage(&mut self, new_state: u256) -> Result<()> {
            Ok(())
        }

        #[ink(message)]
        pub fn respond_to_challenge(&mut self, amount: u64) -> Result<()> {
            Ok(())
        }

        // ---- As referee ----
        #[ink(message)]
        pub fn challenge_provider(&mut self, provider: AccountId) -> Result<()> {
            Ok(())
        }

        #[ink(message)]
        pub fn slash_provider(&mut self, provider: AccountId) -> Result<()> {
            self.only_referee()?;
            Ok(())
        }
    }

    // ---- Utils ----
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {}

    pub type Result<T> = core::result::Result<T, Error>;

    #[cfg(test)]
    mod tests;
}
