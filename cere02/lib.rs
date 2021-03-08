#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod ddc {

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.

    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;

    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        OnlyOwner,
        SameDepositValue,
        NoPermission,
        InsufficientDeposit,
        PidNotFound
    }

    pub type Result<T> = core::result::Result<T, Error>;

    
    #[ink(storage)]
    pub struct Ddc {
        ///Owner of Contract.
        owner: Lazy<AccountId>,
        /// Total token supply.
        minimum_deposit: Lazy<Balance>,
        /// Mapping from owner to number of owned token.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping from ddc wallet to metrics vector
        /// 1st partionId; 2nd dataReceived; 3rd dataReplicated; 4th requestReceived; 5th requestReplicated
        metrics: StorageHashMap<AccountId, Vec<u64>>,
        /// Token symbol in case you want one
        symbol: String,
    }

    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }


    impl Ddc {
        /// Constructor that initializes the contract
        #[ink(constructor)]
        pub fn new(min_deposit: Balance, symbol: String) -> Self {
            let caller = Self::env().caller();
            let balances = StorageHashMap::new();
            let metrics = StorageHashMap::new();

            let instance = Self {
                owner: Lazy::new(caller),
                minimum_deposit: Lazy::new(min_deposit),
                balances,
                metrics,
                symbol
            };
            instance
        }

        /// Returns the minimum deposit required.
        #[ink(message)]
        pub fn minimum_deposit(&self) -> Balance {
            *self.minimum_deposit
        }

        /// Returns the account balance for the specified `owner`.
        ///
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0)
        }

        #[ink(message)]
        pub fn token_symbol(&self) -> String {
            self.symbol.clone()
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, to: AccountId) -> Result<()> {
            self.only_owner(self.env().caller())?;
            *self.owner = to;
            Ok(())
        }

        #[ink(message)]
        pub fn change_minimum_deposit(&mut self, new_value: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;
            self.diff_deposit(new_value)?;

            self.minimum_deposit = Lazy::new(new_value);

            Ok(())
        }

        #[ink(message)]
        pub fn create_payment(&mut self, value: Balance, pid: u64) -> Result<()> {
            let order = self.env().caller();
            let min_value = *self.minimum_deposit;
            if value < min_value {
                return Err(Error::InsufficientDeposit);
            }
            self.balances.insert(order, value);

            //let v = vec![pid,0,0,0,0];

            let mut v = Vec::new();
            v.push(pid);
            v.push(0);
            v.push(0);
            v.push(0);
            v.push(0);
            
            
            self.metrics.insert(order, v);

            Self::env().emit_event(Deposit {
                from: Some(order),
                value: value,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn report_metrics(&mut self, pid: u64, data_rec: u64, data_rep: u64, req_rec: u64, req_rep: u64) -> Result<()> {
            let reporter = self.env().caller();
            let reporter_balance = self.balance_of_or_zero(&reporter);
            if reporter_balance == 0 {
                return Err(Error::NoPermission);
            }
            if pid != self.get_pid(&reporter) {
                return Err(Error::PidNotFound);
            }

            let v = self.metrics.get(&reporter).unwrap();
            let data_received = v[1] + data_rec;
            let data_replicated = v[2] + data_rep;
            let request_received = v[3] + req_rec;
            let request_replicated = v[4] + req_rep;

            // let v2 = vec![pid,data_received,data_replicated,request_received,request_replicated];
            
            let mut v2 = Vec::new();
            v2.push(pid);
            v2.push(data_received);
            v2.push(data_replicated);
            v2.push(request_received);
            v2.push(request_replicated);
            self.metrics.insert(reporter, v2);

            Ok(())
            
        }

        fn only_owner(&self, caller: AccountId) -> Result<()> {
            if *self.owner == caller {
                Ok(())
            } else {
                return Err(Error::OnlyOwner);
            }
        }

        fn diff_deposit(&self, new_value: Balance) -> Result<()> {
            if *self.minimum_deposit != new_value {
                Ok(())
            } else {
                return Err(Error::SameDepositValue);
            }

        }


        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }

        fn get_pid(&self, owner: &AccountId) -> u64 {
            let v = self.metrics.get(owner).unwrap();
            v[0]
        }

        fn get_metrics(&self, owner: &AccountId) -> &Vec<u64> {
            let v = self.metrics.get(owner).unwrap();
            v
        }


    }

    
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            let contract = Ddc::new(99, "DDC".to_string());
            assert_eq!(contract.minimum_deposit(), 99);
            assert_eq!(contract.token_symbol(), "DDC".to_owned());
            assert_ne!(contract.symbol, "NoDDC".to_owned())
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn create_payment_works() {
            let mut contract = Ddc::new(99, "DDC".to_string());
            let payer = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer), 0);
            assert_eq!(contract.create_payment(99, 20),Ok(()));
            assert_eq!(contract.balance_of(payer), 99);
        }

        #[ink::test]
        fn report_metrics_works() {
            let mut contract = Ddc::new(100,  "DDC".to_string());
            let reporter = AccountId::from([0x1; 32]);
            assert_eq!(contract.create_payment(100,10), Ok(()));
            assert_eq!(contract.balance_of(reporter), 100);
            let v = contract.get_metrics(&reporter);
            assert_eq!(v[0],10);
            assert_eq!(v[1],0);
            assert_eq!(v[2],0);
            assert_eq!(v[3],0);
            assert_eq!(v[4],0);
            assert_eq!(contract.report_metrics(10,20,30,40,50), Ok(()));
            let vv = contract.get_metrics(&reporter);
            assert_eq!(vv[0],10);
            assert_eq!(vv[1],20);
            assert_eq!(vv[2],30);
            assert_eq!(vv[3],40);
            assert_eq!(vv[4],50);

        }
    }
}
