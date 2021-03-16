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
        PidNotFound,
        TransferFailed,
        OverLimit
    }

    pub type Result<T> = core::result::Result<T, Error>;

    
    #[ink(storage)]
    pub struct Ddc {
        ///Owner of Contract.
        owner: Lazy<AccountId>,
        /// tier 3 fee in native coins
        /// tier 3 is the minimum, tier 1 is the maximum
        /// example tier 3 fee = 1, tier 2 fee = 10, tier 3 fee = 100
        tier3_fee: Lazy<Balance>,
        /// tier 3 limit, a number
        tier3_limit: u64,
        /// tier 2 fee
        tier2_fee: Lazy<Balance>,
        /// tier 2 limit
        tier2_limit: u64,
        /// tier 1 fee
        tier1_fee: Lazy<Balance>,
        /// tier 1 limit
        tier1_limit: u64,
        /// Mapping from owner to number of owned coins.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping from ddc wallet to metrics vector
        /// 1st tier; 2nd dataReceived; 3rd dataReplicated; 4th requestReceived; 5th requestReplicated
        metrics: StorageHashMap<AccountId, Vec<u64>>,
        /// contract symbol example: "DDC"
        symbol: String,
    }

    /// event emit when a deposit is made
    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }


    impl Ddc {
        /// Constructor that initializes the contract
        /// Give tier3fee, tier3limit, tier2fee, tier2limit, tier1fee, tier1 limit, and a symbol to initialize
        #[ink(constructor)]
        pub fn new(tier3fee: Balance, tier3limit: u64, tier2fee: Balance, tier2limit: u64, tier1fee: Balance, tier1limit: u64, symbol: String) -> Self {
            let caller = Self::env().caller();
            let balances = StorageHashMap::new();
            let metrics = StorageHashMap::new();

            let instance = Self {
                owner: Lazy::new(caller),
                tier3_fee: Lazy::new(tier3fee),
                tier3_limit: tier3limit,
                tier2_fee: Lazy::new(tier2fee),
                tier2_limit: tier2limit,
                tier1_fee: Lazy::new(tier1fee),
                tier1_limit: tier1limit,
                balances,
                metrics,
                symbol
            };
            instance
        }

        /// Given a tier id: 1, 2, 3
        /// return the fee required
        #[ink(message)]
        pub fn tier_deposit(&self, tid: u64) -> Balance {
            if tid == 1 {
                return *self.tier1_fee;
            } else if tid == 2 {
                return *self.tier2_fee;
            } else if tid == 3 {
                return *self.tier3_fee;
            } else {
                return 0 as Balance;
            }
            
        }

        /// Returns the account balance for the specified `account`.
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0)
        }


        /// Return the total balance held in this contract
        #[ink(message)]
        pub fn balance_of_contract(&self) -> Balance {
            self.env().balance()
        }

        /// Return the contract symbol
        #[ink(message)]
        pub fn token_symbol(&self) -> String {
            self.symbol.clone()
        }

        #[ink(message)]
        pub fn metrics_of(&self, acct: AccountId) -> Vec<u64> {
            let v =self.get_metrics(&acct);
            return v.clone();
        }

        /// Return the tier id corresponding to the account
        #[ink(message)]
        pub fn tier_id_of(&self, acct: AccountId) -> u64 {
            let tid = self.get_tier_id(&acct);
            tid
        }

        /// Return the tier limit corresponding the account
        #[ink(message)]
        pub fn tier_limit_of(&self, acct: AccountId) -> u64 {
            let tid = self.get_tier_id(&acct);
            let tl = self.get_tier_limit(tid);
            tl.clone()
        }

        /// Transfer the contract admin to the accoung provided
        #[ink(message)]
        pub fn transfer_ownership(&mut self, to: AccountId) -> Result<()> {
            self.only_owner(self.env().caller())?;
            *self.owner = to;
            Ok(())
        }

        // #[ink(message)]
        // pub fn change_minimum_deposit(&mut self, new_value: Balance) -> Result<()> {
        //     let caller = self.env().caller();
        //     self.only_owner(caller)?;
        //     // self.diff_deposit(new_value)?;

        //     self.tier3_fee = Lazy::new(new_value);

        //     Ok(())
        // }

        /// change the tier fee given the tier id and new fee 
        /// Must be the contract admin to call this function
        #[ink(message)]
        pub fn change_tier_fee(&mut self, tier_id: u64, new_fee: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;
            self.diff_deposit(tier_id, new_fee)?;
            if tier_id == 1 {
                self.tier1_fee = Lazy::new(new_fee);
                return Ok(());
            } else if tier_id == 2 {
                self.tier2_fee = Lazy::new(new_fee);
                return Ok(());
            } else if tier_id == 3 {
                self.tier3_fee = Lazy::new(new_fee);
                return Ok(());
            } else {
                return Err(Error::NoPermission);
            }
        }


        /// Change tier limit given tier id and a new limit
        /// Must be contract admin to call this function
        #[ink(message)]
        pub fn change_tier_limit(&mut self, tier_id: u64, new_limit: u64) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;
            // self.diff_deposit(tier_id, new_fee)?;
            if tier_id == 1 && self.tier1_limit != new_limit {
                self.tier1_limit = new_limit;
                return Ok(());
            } else if tier_id == 2 && self.tier2_limit != new_limit {
                self.tier2_limit = new_limit;
                return Ok(());
            } else if tier_id == 3 && self.tier3_limit != new_limit {
                self.tier3_limit = new_limit;
                return Ok(());
            } else {
                return Err(Error::NoPermission);
            }
        }
        
        /// Receive payment from the participating DDC node
        /// Store payment into users balance map
        /// Initialize user metrics map
        #[ink(message)]
        pub fn create_payment(&mut self, value: Balance) -> Result<()> {
            let payer = self.env().caller();
            // let min_value = *self.minimum_deposit;
            // if value < min_value {
            //     return Err(Error::InsufficientDeposit);
            // }
            if *self.tier3_fee == value {
                self.balances.insert(payer, value);
                let mut v = Vec::new();
                v.push(3); // tier 3

                for _i in 0..4 {
                    v.push(0);
                }

                self.metrics.insert(payer, v);

                self.env().emit_event(Deposit{
                    from: Some(payer),
                    value: value,
                });

                return Ok(());

            } else if *self.tier2_fee == value {
                self.balances.insert(payer, value);
                let mut v = Vec::new();
                v.push(2); // tier 2

                for _i in 0..4 {
                    v.push(0);
                }

                self.metrics.insert(payer, v);

                self.env().emit_event(Deposit{
                    from: Some(payer),
                    value: value,
                });
                
                return Ok(());

            } else if *self.tier1_fee == value {
                self.balances.insert(payer, value);
                let mut v = Vec::new();
                v.push(1); // tier 1

                for _i in 0..4 {
                    v.push(0);
                }

                self.metrics.insert(payer, v);

                self.env().emit_event(Deposit{
                    from: Some(payer),
                    value: value,
                });

                return Ok(());

            } else {
                return Err(Error::InsufficientDeposit);
            }
            
        }

        /// Take metrics reported by DDC nodes
        /// Insert metrics to the reporting node's map in the contract
        #[ink(message)]
        pub fn report_metrics(&mut self, data_rec: u64, data_rep: u64, req_rec: u64, req_rep: u64) -> Result<()> {
            let reporter = self.env().caller();
            let reporter_balance = self.balance_of_or_zero(&reporter);
            if reporter_balance == 0 {
                return Err(Error::NoPermission);
            }
            let tier_id = self.get_tier_id(&reporter);

            let tier_limit = self.get_tier_limit(tier_id);
                        
            let v = self.metrics.get(&reporter).unwrap();
            let d_rec = v[1] + data_rec;
            let d_rep = v[2] + data_rep;
            let r_rec = v[3] + req_rec;
            let r_rep = v[4] + req_rep;

            if d_rec <= tier_limit && d_rep <= tier_limit && r_rec <=tier_limit && r_rep <= tier_limit {
                let mut v2 = Vec::new();
                v2.push(tier_id);
                v2.push(d_rec);
                v2.push(d_rep);
                v2.push(r_rec);
                v2.push(r_rep);
                self.metrics.insert(reporter, v2);

                return Ok(());
            } else {
                return Err(Error::OverLimit);
            }
                    
        }

        /// DDC node can call this function to opt out
        /// Refund the DDC node
        /// Clear the node's balance inside the contract
        /// But keep the metrics record


        #[ink(message)]
        pub fn opt_out(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let caller_bal = self.balance_of_or_zero(&caller) as Balance;

            if caller_bal == 0 {
                return Err(Error::InsufficientDeposit);
            }

            self.balances.insert(caller, 0);

            // ink! transfer emit a panic!, this function doesn't work with this nightly build
            // self.env().transfer(caller, balance).expect("pay out failure");

            let _result = match self.env().transfer(caller, caller_bal) {
                Err(_e) => Err(Error::TransferFailed),
                Ok(_v) => Ok(()),
            };

            Ok(())       

        }

        //TODO:  transfer funds out of contract



        /// Check if account is the owner of this contract
        fn only_owner(&self, caller: AccountId) -> Result<()> {
            if *self.owner == caller {
                Ok(())
            } else {
                return Err(Error::OnlyOwner);
            }
        }

        /// Check if the new fee is the same as the old fee
        /// Return error if they are the same
        fn diff_deposit(&self, tid: u64, new_value: Balance) -> Result<()> {
            if tid == 3 && *self.tier3_fee != new_value {
                return Ok(());
            } else if tid == 2 && *self.tier2_fee != new_value {
                return Ok(());
            } else if tid == 1 && *self.tier1_fee != new_value {
                return Ok(());
            }else {
                return Err(Error::SameDepositValue);
            }

        }

        /// Return balance of an account
        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }


        /// Return tier id given an account
        fn get_tier_id(&self, owner: &AccountId) -> u64 {
            let v = self.metrics.get(owner).unwrap();
            v[0]
        }

        /// Return metrics given an account
        fn get_metrics(&self, owner: &AccountId) -> &Vec<u64> {
            let v = self.metrics.get(owner).unwrap();
            v
        }

        /// Return tier limit given a tier id 1.2.3
        fn get_tier_limit(&self, tid: u64) -> u64 {
            if tid == 1 {
                return self.tier1_limit;
            } else if tid == 2 {
                return self.tier2_limit;
            } else if tid == 3 {
                return self.tier3_limit;
            } else {
                return 0;
            }
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
            let contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            assert_eq!(contract.tier_deposit(1), 8);
            assert_eq!(contract.tier_deposit(2), 4);
            assert_eq!(contract.tier_deposit(3), 2);
            assert_eq!(contract.token_symbol(), "DDC".to_owned());
            assert_ne!(contract.symbol, "NoDDC".to_owned())
        }

        /// Test if a function can only be called by the contract admin
        #[ink::test]
        fn onlyowner_works() {
            let contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
        }

        /// Test that we can transfer owner to another account
        #[ink::test]
        fn transfer_ownership_works() {
            let mut contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            contract
                .transfer_ownership(AccountId::from([0x0; 32]))
                .unwrap();
            assert_eq!(contract.only_owner(AccountId::from([0x0; 32])), Ok(()));
        }

        /// We test the contract can take payment from users
        #[ink::test]
        fn create_payment_works() {
            let mut contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            let payer = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer), 0);
            assert_eq!(contract.create_payment(2),Ok(()));
            assert_eq!(contract.balance_of(payer), 2);
        }

        /// We test the contract can process the metrics reported by DDC
        #[ink::test]
        fn report_metrics_works() {
            let mut contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            let reporter = AccountId::from([0x1; 32]);
            assert_eq!(contract.create_payment(8), Ok(()));
            assert_eq!(contract.balance_of(reporter), 8);
            let v = contract.get_metrics(&reporter);
            assert_eq!(v[0],1);
            assert_eq!(v[1],0);
            assert_eq!(v[2],0);
            assert_eq!(v[3],0);
            assert_eq!(v[4],0);
            assert_eq!(contract.report_metrics(100,200,300,400), Ok(()));
            let vv = contract.get_metrics(&reporter);
            assert_eq!(vv[0],1);
            assert_eq!(vv[1],100);
            assert_eq!(vv[2],200);
            assert_eq!(vv[3],300);
            assert_eq!(vv[4],400);
        }

        /// Can read metrics 
        #[ink::test]
        fn read_metrics_works() {
            let mut contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            let reporter = AccountId::from([0x1; 32]);
            assert_eq!(contract.create_payment(4), Ok(()));
            assert_eq!(contract.balance_of(reporter), 4);
            let v = contract.metrics_of(reporter);
            assert_eq!(v[0],2);
            assert_eq!(v[1],0);
            assert_eq!(v[2],0);
            assert_eq!(v[3],0);
            assert_eq!(v[4],0);
            assert_eq!(contract.report_metrics(20,30,40,50), Ok(()));
            let vv = contract.metrics_of(reporter);
            assert_eq!(vv[0],2);
            assert_eq!(vv[1],20);
            assert_eq!(vv[2],30);
            assert_eq!(vv[3],40);
            assert_eq!(vv[4],50);
        }

        /// Test DDC node can opt out the program and get refund
        #[ink::test]
        fn opt_out_works() {
            let mut contract = Ddc::new(2, 2000,4, 4000, 8, 3000,"DDC".to_string());
            let payer = AccountId::from([0x1; 32]);
            assert_eq!(contract.create_payment(8), Ok(()));
            assert_eq!(contract.balance_of(payer), 8);
            assert_eq!(contract.opt_out(), Ok(()));
            assert_eq!(contract.balance_of(payer), 0);
        }
    }
}
