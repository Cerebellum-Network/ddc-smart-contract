#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod ddc {
    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;


    // ---- Storage ----
    #[ink(storage)]
    pub struct Ddc {
        ///Owner of Contract.
        owner: Lazy<AccountId>,
        /// HashMap of tier_id: vector of [tier_id, tier_fee, tier_throughput_limit, tier_storage_limit]
        service: StorageHashMap<u128, Vec<u128>>,
        /// Mapping from owner to number of owned coins.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping from ddc wallet to metrics vector
        /// 1st tier; 2nd dataReceived; 3rd dataReplicated; 4th requestReceived; 5th requestReplicated
        metrics: StorageHashMap<AccountId, Vec<u128>>,
        /// contract symbol example: "DDC"
        symbol: String,
        /// contract status
        pause: bool,
    }

    impl Ddc {
        /// Constructor that initializes the contract
        /// Give tier3fee, tier3limit, tier2fee, tier2limit, tier1fee, tier1 limit, and a symbol to initialize
        #[ink(constructor)]
        pub fn new(
            tier3fee: Balance,
            tier3_throughput_limit: u128,
            tier3_storage_limit: u128,
            tier2fee: Balance,
            tier2_throughput_limit: u128,
            tier2_storage_limit: u128,
            tier1fee: Balance,
            tier1_throughput_limit: u128,
            tier1_storage_limit: u128,
            symbol: String) -> Self {
            let caller = Self::env().caller();
            let balances = StorageHashMap::new();
            let metrics = StorageHashMap::new();

            let mut service_v = StorageHashMap::new();

            let mut t1 = Vec::new();

            t1.push(1);
            t1.push(tier1fee);
            t1.push(tier1_throughput_limit);
            t1.push(tier1_storage_limit);

            service_v.insert(1, t1);

            let mut t2 = Vec::new();

            t2.push(2);
            t2.push(tier2fee);
            t2.push(tier2_throughput_limit);
            t2.push(tier2_storage_limit);

            service_v.insert(2, t2);

            let mut t3 = Vec::new();

            t3.push(3);
            t3.push(tier3fee);
            t3.push(tier3_throughput_limit);
            t3.push(tier3_storage_limit);

            service_v.insert(3, t3);

            let instance = Self {
                owner: Lazy::new(caller),
                service: service_v,
                balances,
                metrics,
                symbol,
                pause: false,
            };
            instance
        }
    }


    // ---- Admin ----
    impl Ddc {
        /// Check if account is the owner of this contract
        fn only_owner(&self, caller: AccountId) -> Result<()> {
            if *self.owner == caller {
                Ok(())
            } else {
                return Err(Error::OnlyOwner);
            }
        }

        /// Transfer the contract admin to the accoung provided
        #[ink(message)]
        pub fn transfer_ownership(&mut self, to: AccountId) -> Result<()> {
            self.only_active()?;
            self.only_owner(self.env().caller())?;
            *self.owner = to;
            Ok(())
        }
    }


    // ---- Admin: Funds ----
    impl Ddc {
        // This seems to be the endowment you give to the contract upon initializing it
        // Official recommendation is 1000
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

        /// Given a destination account, transfer all the contract balance to it
        /// only contract owner can call this function
        /// destination account can be the same as the contract owner
        /// return OK or an error
        #[ink(message)]
        pub fn transfer_all_balance(&mut self, destination: AccountId) -> Result<()> {
            self.only_not_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;
            let contract_bal = self.env().balance();
            // ink! transfer emit a panic!, the function below doesn't work, at least with this nightly build
            // self.env().transfer(destination, contract_bal).expect("pay out failure");

            let _result = match self.env().transfer(destination, contract_bal) {
                Err(_e) => Err(Error::TransferFailed),
                Ok(_v) => Ok(()),
            };

            Ok(())
        }

        /// given an account id, revoke its membership by clearing its balance;
        /// only the contract owner can call this function
        /// return ok or error
        #[ink(message)]
        pub fn revoke_membership(&mut self, member: AccountId) -> Result<()> {
            self.only_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;
            let member_bal = self.balance_of_or_zero(&member) as Balance;
            if member_bal == 0 {
                return Err(Error::ZeroBalance);
            }
            // clear the balance, but keeps the metrics record
            self.balances.insert(member, 0);
            Ok(())
        }
    }


    // ---- Admin: Pausable ----
    impl Ddc {
        #[ink(message)]
        pub fn paused_or_not(&self) -> bool {
            self.pause
        }

        /// check if contract is active
        /// return ok if pause is false - not paused
        fn only_active(&self) -> Result<()> {
            if self.pause == false {
                Ok(())
            } else {
                return Err(Error::ContractPaused);
            }
        }

        fn only_not_active(&self) -> Result<()> {
            if self.pause == true {
                Ok(())
            } else {
                return Err(Error::ContractActive);
            }
        }

        /// flip the status of contract, pause it if it is live
        /// unpause it if it is paused before
        /// only contract owner can call this function
        #[ink(message)]
        pub fn flip_contract_status(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;
            let status = self.pause;
            if status == false {
                self.pause = true;
                Ok(())
            } else {
                self.pause = false;
                Ok(())
            }
        }
    }


    // ---- Admin: Tiers ----

    // #[derive(scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    // #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    // pub struct ServiceTier{
    //     tier_id: u128,
    //     tier_fee: u128,
    //     throughput_limit: u128,
    //     storage_limit: u128,
    // }

    // impl ServiceTier {
    //     pub fn new(tier_id: u128, tier_fee: u128, throughput_limit: u128, storage_limit: u128) -> ServiceTier {

    //         ServiceTier {
    //             tier_id,
    //             tier_fee,
    //             throughput_limit,
    //             storage_limit
    //         }
    //     }
    // }

    impl Ddc {
        /// Given a tier id: 1, 2, 3
        /// return the fee required
        #[ink(message)]
        pub fn tier_deposit(&self, tid: u128) -> Balance {
            //self.tid_in_bound(tier_id)?;
            if tid > 3 {
                return 0 as Balance;
            }
            let v = self.service.get(&tid).unwrap();
            return v[1] as Balance;
        }

        #[ink(message)]
        pub fn get_all_tiers(&self) -> Vec<u128> {
            let mut v = Vec::new();
            // v1 = [tier_id, tier_fee, tier_throughput_limit, tier_storage_limit]
            let v1 = self.service.get(&1).unwrap();

            let v2 = self.service.get(&2).unwrap();

            let v3 = self.service.get(&3).unwrap();

            for i in 0..4 {
                v.push(v1[i]);
            }
            for j in 0..4 {
                v.push(v2[j]);
            }
            for k in 0..4 {
                v.push(v3[k]);
            }
            v
        }

        /// check if tid is within 1, 2 ,3
        /// return ok or error
        fn tid_in_bound(&self, tid: u128) -> Result<()> {
            if tid <= 3 {
                Ok(())
            } else {
                return Err(Error::TidOutOfBound);
            }
        }

        /// change the tier fee given the tier id and new fee
        /// Must be the contract admin to call this function
        #[ink(message)]
        pub fn change_tier_fee(&mut self, tier_id: u128, new_fee: Balance) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;
            // let n_f = new_fee as u128;

            self.diff_deposit(tier_id, new_fee)?;

            // v[0] index, v[1] fee, v[2] throughput_limit, v[3] storage_limit
            let v = self.service.get(&tier_id).unwrap();

            let mut v2 = Vec::new();
            v2.push(v[0]);
            v2.push(new_fee);
            v2.push(v[2]);
            v2.push(v[3]);

            self.service.insert(tier_id, v2);
            Ok(())
        }

        /// Change tier limit given tier id and a new limit
        /// Must be contract admin to call this function
        #[ink(message)]
        pub fn change_tier_limit(&mut self, tier_id: u128, new_throughput_limit: u128, new_storage_limit: u128) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;
            // v[0] index, v[1] fee, v[2] throughput_limit, v[3] storage_limit
            let v = self.service.get(&tier_id).unwrap();
            let mut v2 = Vec::new();
            v2.push(v[0]);
            v2.push(v[1]);
            v2.push(new_throughput_limit);
            v2.push(new_storage_limit);
            self.service.insert(tier_id, v2);
            Ok(())
        }

        /// Check if the new fee is the same as the old fee
        /// Return error if they are the same
        fn diff_deposit(&self, tid: u128, new_value: Balance) -> Result<()> {
            self.tid_in_bound(tid)?;
            let newv = new_value as u128;
            let v = self.service.get(&tid).unwrap();
            if v[1] != newv {
                return Ok(());
            } else {
                return Err(Error::SameDepositValue);
            }
        }

        /// Return tier limit given a tier id 1, 2, 3
        fn get_tier_limit(&self, tid: u128) -> Vec<u128> {
            let mut v = Vec::new();
            let v2 = self.service.get(&tid).unwrap();
            let throughput_limit = v2[2];
            let storage_limit = v2[3];
            v.push(throughput_limit);
            v.push(storage_limit);
            v
        }
    }


    // ---- Apps Payments ----

    /// event emit when a deposit is made
    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    impl Ddc {
        /// Returns the account balance for the specified `account`.
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0)
        }

        #[ink(message)]
        pub fn metrics_of(&self, acct: AccountId) -> Vec<u128> {
            let v = self.get_metrics(&acct);
            return v.clone();
        }

        /// Return the tier id corresponding to the account
        #[ink(message)]
        pub fn tier_id_of(&self, acct: AccountId) -> u128 {
            let tid = self.get_tier_id(&acct);
            tid
        }

        /// Return the tier limit corresponding the account
        #[ink(message)]
        pub fn tier_limit_of(&self, acct: AccountId) -> Vec<u128> {
            let tid = self.get_tier_id(&acct);
            let tl = self.get_tier_limit(tid);
            tl.clone()
        }

        /// Return balance of an account
        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }

        /// Return tier id given an account
        fn get_tier_id(&self, owner: &AccountId) -> u128 {
            let v = self.metrics.get(owner).unwrap();
            v[0]
        }

        /// Receive payment from the participating DDC node
        /// Store payment into users balance map
        /// Initialize user metrics map
        #[ink(message, payable)]
        pub fn subscribe(&mut self, tier_id: u128) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let payer = self.env().caller();
            let value = self.env().transferred_balance();
            let fee_value = value as u128;
            let service_v = self.service.get(&tier_id).unwrap();
            if service_v[1] > fee_value {
                return Err(Error::InsufficientDeposit);
            }

            // TODO: Incorrect overwriting, losing tokens if multiple calls.
            self.balances.insert(payer, value);
            let mut v = Vec::new();
            v.push(tier_id); // tier_id 1,2,3
            for _i in 0..4 {
                v.push(0);
            }
            self.metrics.insert(payer, v);
            self.env().emit_event(Deposit {
                from: Some(payer),
                value: value,
            });

            return Ok(());
        }

        /// DDC node can call this function to opt out
        /// Refund the DDC node
        /// Clear the node's balance inside the contract
        /// But keep the metrics record
        #[ink(message)]
        pub fn unsubscribe(&mut self) -> Result<()> {
            self.only_active()?;
            let caller = self.env().caller();
            let caller_bal = self.balance_of_or_zero(&caller) as Balance;

            if caller_bal == 0 {
                return Err(Error::ZeroBalance);
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
    }


    // ---- Metrics Reporting ----
    impl Ddc {
        /// Return metrics given an account
        fn get_metrics(&self, owner: &AccountId) -> &Vec<u128> {
            let v = self.metrics.get(owner).unwrap();
            v
        }

        /// Take metrics reported by DDC nodes
        /// Insert metrics to the reporting node's map in the contract
        #[ink(message)]
        pub fn report_metrics(&mut self, data_rec: u128, data_rep: u128, req_rec: u128, req_rep: u128) -> Result<()> {
            self.only_active()?;
            let reporter = self.env().caller();
            let reporter_balance = self.balance_of_or_zero(&reporter);
            if reporter_balance == 0 {
                return Err(Error::NoPermission);
            }
            let tier_id = self.get_tier_id(&reporter);

            let tier_limit = self.get_tier_limit(tier_id);

            let t_thr_limit = tier_limit[0];  // throughput is for request received/replicated
            let t_sto_limit = tier_limit[1];  // storage is data received/replicated

            let v = self.metrics.get(&reporter).unwrap();
            let d_rec = v[1] + data_rec;
            let d_rep = v[2] + data_rep;
            let r_rec = v[3] + req_rec;
            let r_rep = v[4] + req_rep;

            if d_rec <= t_sto_limit && d_rep <= t_sto_limit && r_rec <= t_thr_limit && r_rep <= t_thr_limit {
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
    }


    // ---- Utils ----
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        OnlyOwner,
        SameDepositValue,
        NoPermission,
        InsufficientDeposit,
        TransferFailed,
        ZeroBalance,
        OverLimit,
        TidOutOfBound,
        ContractPaused,
        ContractActive,
    }

    pub type Result<T> = core::result::Result<T, Error>;


    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            let contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.tier_deposit(1), 8);
            assert_eq!(contract.tier_deposit(2), 4);
            assert_eq!(contract.tier_deposit(3), 2);
            assert_eq!(contract.token_symbol(), "DDC".to_owned());
            assert_ne!(contract.symbol, "NoDDC".to_owned())
        }


        /// Test if a function can only be called by the contract admin
        #[ink::test]
        fn onlyowner_works() {
            let contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
        }

        /// Test that we can transfer owner to another account
        #[ink::test]
        fn transfer_ownership_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            contract
                .transfer_ownership(AccountId::from([0x0; 32]))
                .unwrap();
            assert_eq!(contract.only_owner(AccountId::from([0x0; 32])), Ok(()));
        }

        /// Test the contract can take payment from users
        #[ink::test]
        fn subscribe_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let payer = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer), 0);
            assert_eq!(contract.subscribe(3), Ok(()));
            // assert_eq!(contract.balance_of(payer), 2);
        }

        /// Test the total balance of the contract is correct
        #[ink::test]
        fn balance_of_contract_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let payer_one = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer_one), 0);
            assert_eq!(contract.subscribe(3), Ok(()));
            assert_eq!(contract.balance_of_contract(), 0);
        }

        /// Test the contract can return the correct tier if given an account id
        #[ink::test]
        fn tier_id_of_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let payer_one = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer_one), 0);
            assert_eq!(contract.subscribe(2), Ok(()));
            assert_eq!(contract.tier_id_of(payer_one), 2);
        }

        /// Test we can read metrics 
        #[ink::test]
        fn get_all_tiers_works() {
            let contract = Ddc::new(2000, 2000, 2000, 4000, 4000, 4000, 8000, 8000, 8000, "DDC".to_string());

            let v = contract.get_all_tiers();
            assert_eq!(v[0], 1); //tid
            assert_eq!(v[1], 8000); //fee
            assert_eq!(v[2], 8000); //throughput limit
            assert_eq!(v[3], 8000); // storage limit
            assert_eq!(v[4], 2); //tid
            assert_eq!(v[5], 4000); //t2 fee
            assert_eq!(v[6], 4000); //t2 throughtput limit
            assert_eq!(v[7], 4000); //t2 storage limit
            assert_eq!(v[8], 3);
            assert_eq!(v[9], 2000);
            assert_eq!(v[10], 2000);
            assert_eq!(v[11], 2000);
        }

        /// Test the contract owner can change tier fees for all 3 tiers
        #[ink::test]
        fn change_tier_fee_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            assert_eq!(contract.change_tier_fee(3, 3), Ok(()));
            assert_eq!(contract.change_tier_fee(2, 5), Ok(()));
            assert_eq!(contract.change_tier_fee(1, 9), Ok(()));
            assert_eq!(contract.tier_deposit(3), 3);
            assert_eq!(contract.tier_deposit(2), 5);
            assert_eq!(contract.tier_deposit(1), 9);
        }

        /// Test the contract can change tier limits for all 3 tiers
        #[ink::test]
        fn change_tier_limit_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            assert_eq!(contract.change_tier_limit(3, 100, 100), Ok(()));
            assert_eq!(contract.change_tier_limit(2, 200, 200), Ok(()));
            assert_eq!(contract.change_tier_limit(1, 300, 300), Ok(()));
            assert_eq!(contract.get_tier_limit(3), vec![100, 100]);
            assert_eq!(contract.get_tier_limit(2), vec![200, 200]);
            assert_eq!(contract.get_tier_limit(1), vec![300, 300]);
        }

        /// Test the contract owner can revoke the membership of a subscriber (a participating ddc node)
        #[ink::test]
        fn revoke_membership_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let payer_one = AccountId::from([0x1; 32]);
            assert_eq!(contract.subscribe(2), Ok(()));
            assert_eq!(contract.revoke_membership(payer_one), Ok(()));
            assert_eq!(contract.balance_of(payer_one), 0);
        }

        /// Test the contract owner can flip the status of the contract
        /// Can pause and unpause the contract
        #[ink::test]
        fn flip_contract_status_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            assert_eq!(contract.paused_or_not(), false);
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), true);
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), false);
        }

        /// Test the contract owner can transfer all the balance out of the contract after it is paused
        #[ink::test]
        fn transfer_all_balance_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            assert_eq!(contract.subscribe(3), Ok(()));
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), true);
            assert_eq!(contract.transfer_all_balance(AccountId::from([0x0; 32])), Ok(()));
            assert_eq!(contract.balance_of_contract(), 0);
        }

        /// Test the contract can process the metrics reported by DDC
        #[ink::test]
        fn report_metrics_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let reporter = AccountId::from([0x1; 32]);
            assert_eq!(contract.subscribe(1), Ok(()));
            assert_eq!(contract.balance_of(reporter), 8);
            let v = contract.get_metrics(&reporter);
            assert_eq!(v[0], 1);
            assert_eq!(v[1], 0);
            assert_eq!(v[2], 0);
            assert_eq!(v[3], 0);
            assert_eq!(v[4], 0);
            assert_eq!(contract.report_metrics(100, 200, 300, 400), Ok(()));
            let vv = contract.get_metrics(&reporter);
            assert_eq!(vv[0], 1);
            assert_eq!(vv[1], 100);
            assert_eq!(vv[2], 200);
            assert_eq!(vv[3], 300);
            assert_eq!(vv[4], 400);
        }

        /// Test we can read metrics 
        #[ink::test]
        fn read_metrics_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let reporter = AccountId::from([0x1; 32]);
            assert_eq!(contract.subscribe(2), Ok(()));
            assert_eq!(contract.balance_of(reporter), 4);
            let v = contract.metrics_of(reporter);
            assert_eq!(v[0], 2);
            assert_eq!(v[1], 0);
            assert_eq!(v[2], 0);
            assert_eq!(v[3], 0);
            assert_eq!(v[4], 0);
            assert_eq!(contract.report_metrics(20, 30, 40, 50), Ok(()));
            let vv = contract.metrics_of(reporter);
            assert_eq!(vv[0], 2);
            assert_eq!(vv[1], 20);
            assert_eq!(vv[2], 30);
            assert_eq!(vv[3], 40);
            assert_eq!(vv[4], 50);
        }

        /// Test DDC node can opt out the program and get refund
        #[ink::test]
        fn unsubscribe_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());
            let payer = AccountId::from([0x1; 32]);
            assert_eq!(contract.subscribe(3), Ok(()));
            assert_eq!(contract.balance_of(payer), 8);
            assert_eq!(contract.unsubscribe(), Ok(()));
            assert_eq!(contract.balance_of(payer), 0);
        }
    }
}
