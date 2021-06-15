#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod ddc_coordinator {
    use ink_storage::collections::HashMap;

    const TIMEOUT: u64 = 60 * 60 * 1000;

    #[ink(storage)]
    pub struct DdcCoordinator {
        locked: bool,
        lock_owner: AccountId,
        updated_at: u64,
    }

    impl DdcCoordinator {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                locked: false,
                lock_owner: AccountId::default(),
                updated_at: 0,
            }
        }

        #[ink(message)]
        pub fn lock(&mut self) -> bool {
            if self.is_locked() {
                false // Lock is already taken.
            } else {
                self.locked = true;
                self.lock_owner = self.env().caller();
                let now = Self::env().block_timestamp();
                self.updated_at = now;
                true // Lock acquired.
            }
        }

        #[ink(message)]
        pub fn unlock(&mut self) {
            if self.lock_owner == self.env().caller() {
                self.locked = false;
            }
        }

        #[ink(message)]
        pub fn is_locked(&self) -> bool {
            let now = Self::env().block_timestamp();
            self.locked && self.updated_at + TIMEOUT > now
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut ddc_coordinator = DdcCoordinator::new();
            assert_eq!(ddc_coordinator.is_locked(), false);
            ddc_coordinator.lock();
            assert_eq!(ddc_coordinator.is_locked(), true);
            ddc_coordinator.unlock();
            assert_eq!(ddc_coordinator.is_locked(), false);
        }
    }
}
