#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod ddc {
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        lazy::Lazy,
        traits::{PackedLayout, SpreadLayout,
        },
    };
    use scale::{Decode, Encode};

    // ---- Storage ----
    #[ink(storage)]
    pub struct Ddc {
        // -- Admin --
        /// Owner of Contract.
        owner: Lazy<AccountId>,
        pause: bool,
        /// contract symbol example: "DDC"
        symbol: String,

        // -- Tiers --
        /// HashMap of tier_id: vector of [tier_id, tier_fee, tier_throughput_limit, tier_storage_limit]
        service: StorageHashMap<u128, Vec<u128>>,

        // -- App Subscriptions --
        /// Mapping from owner to number of owned coins.
        balances: StorageHashMap<AccountId, Balance>,
        subscriptions: StorageHashMap<AccountId, AppSubscription>,

        // -- Admin: Reporters --
        reporters: StorageHashMap<AccountId, ()>,

        // -- Metrics Reporting --
        pub metrics: StorageHashMap<MetricKey, MetricValue>,
        current_period_ms: u64,
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

            // TODO(Aurel): check the unit is milliseconds (not documented).
            let today_ms = (Self::env().block_timestamp() as u64) % MS_PER_DAY;

            let instance = Self {
                owner: Lazy::new(caller),
                service: service_v,
                balances: StorageHashMap::new(),
                subscriptions: StorageHashMap::new(),
                reporters: StorageHashMap::new(),
                metrics: StorageHashMap::new(),
                current_period_ms: today_ms,
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

            let subscription_opt = self.subscriptions.get(&member);

            if subscription_opt.is_none() {
                return Err(Error::NoSubscription);
            }

            let mut subscription = subscription_opt.unwrap().clone();

            if subscription.balance == 0 {
                return Err(Error::ZeroBalance);
            }

            subscription.balance = 0;
            self.subscriptions.insert(member, subscription);

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


    // ---- App Subscriptions ----

    /// event emit when a deposit is made
    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    #[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct AppSubscription {
        start_date_ms: u64,
        end_date_ms: u64,
        tier_id: u128,
        balance: Balance,
    }

    impl Ddc {
        /// Returns the account balance for the specified `account`.
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&mut self, owner: AccountId) -> Balance {
            let subscription_opt = self.subscriptions.get(&owner);

            if subscription_opt.is_none() {
                return 0;
            }

            let subscription = subscription_opt.unwrap();
            subscription.balance
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

        /// Return tier id given an account
        fn get_tier_id(&self, owner: &AccountId) -> u128 {
            let subscription = self.subscriptions.get(owner).unwrap();
            subscription.tier_id
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
            if service_v[1] > fee_value { //TODO: We probably need to summarize the existing balance with provided, in case app wants to deposit more than monthly amount
                return Err(Error::InsufficientDeposit);
            }

            let subscription_opt = self.subscriptions.get(&payer);
            let now = Self::env().block_timestamp();
            let mut subscription: AppSubscription;

            if subscription_opt.is_none() || subscription_opt.unwrap().end_date_ms < now {
                subscription = AppSubscription { start_date_ms: now, end_date_ms: now + 31 * MS_PER_DAY, tier_id, balance: value };
            } else {
                subscription = subscription_opt.unwrap().clone();

                subscription.end_date_ms += 31 * MS_PER_DAY;
                subscription.balance = subscription.balance + value;
            }

            self.subscriptions.insert(payer, subscription);
            self.env().emit_event(Deposit {
                from: Some(payer),
                value: value,
            });

            return Ok(());
        }

        // DDC node can call this function to opt out
        // Refund the DDC node
        // Clear the node's balance inside the contract
        // But keep the metrics record

//        #[ink(message)]
        // TODO: Need to re-design
//        pub fn unsubscribe(&mut self) -> Result<()> {
//            self.only_active()?;
//            let caller = self.env().caller();
//            let caller_bal = self.balance_of_or_zero(&caller) as Balance;
//
//            if caller_bal == 0 {
//                return Err(Error::ZeroBalance);
//            }
//
//            let subscription_opt = self.subscriptions.get(&caller);
//
//            if subscription_opt.is_none() {
//                return Err(Error::NoSubscription);
//            }
//
//            let mut subscription = subscription_opt.unwrap().clone();
//
//            subscription.balance = 0;
//            self.subscriptions.insert(caller, subscription);
//
//            // ink! transfer emit a panic!, this function doesn't work with this nightly build
//            // self.env().transfer(caller, balance).expect("pay out failure");
//
//            let _result = match self.env().transfer(caller, caller_bal) {
//                Err(_e) => Err(Error::TransferFailed),
//                Ok(_v) => Ok(()),
//            };
//
//            Ok(())
//        }
    }


    // ---- Admin: Reporters ----

    #[ink(event)]
    pub struct ReporterAdded {
        #[ink(topic)]
        reporter: AccountId,
    }

    #[ink(event)]
    pub struct ReporterRemoved {
        #[ink(topic)]
        reporter: AccountId,
    }

    #[ink(event)]
    pub struct ErrorOnlyReporter {}

    impl Ddc {
        /// Check if account is an approved reporter.
        fn only_reporter(&self, caller: &AccountId) -> Result<()> {
            if self.is_reporter(*caller) {
                Ok(())
            } else {
                self.env().emit_event(ErrorOnlyReporter {});
                Err(Error::OnlyReporter)
            }
        }

        #[ink(message)]
        pub fn is_reporter(&self, reporter: AccountId) -> bool {
            self.reporters.contains_key(&reporter)
        }

        #[ink(message)]
        pub fn add_reporter(&mut self, reporter: AccountId) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            self.reporters.insert(reporter, ());
            Self::env().emit_event(ReporterAdded { reporter });
            Ok(())
        }

        #[ink(message)]
        pub fn remove_reporter(&mut self, reporter: AccountId) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            self.reporters.take(&reporter);
            Self::env().emit_event(ReporterRemoved { reporter });
            Ok(())
        }
    }

    // ---- Metrics Reporting ----
    #[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricKey {
        app_id: AccountId,
        day_of_month: u64,
    }

    #[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricValue {
        stored_bytes: u128,
        requests: u128,
    }

    impl MetricValue {
        pub fn add_assign(&mut self, other: &Self) {
            self.stored_bytes += other.stored_bytes;
            self.requests += other.requests;
        }

        pub fn max_assign(&mut self, other: &Self) {
            if self.stored_bytes < other.stored_bytes {
                self.stored_bytes = other.stored_bytes;
            }
            if self.requests < other.requests {
                self.requests = other.requests;
            }
        }
    }

    #[ink(event)]
    pub struct NewMetric {
        #[ink(topic)]
        reporter: AccountId,
        #[ink(topic)]
        key: MetricKey,
        metrics: MetricValue,
    }

    #[ink(event)]
    pub struct MetricPeriodFinalized {
        #[ink(topic)]
        reporter: AccountId,
        start_ms: u64,
    }


    impl Ddc {
        #[ink(message)]
        pub fn metrics_since_subscription(&self, app_id: AccountId) -> Result<MetricValue> {
            let subscription = self.subscriptions.get(&app_id)
                .ok_or(Error::NoSubscription)?;

            let now_ms = Self::env().block_timestamp() as u64;
            let metrics = self.metrics_for_period(app_id, subscription.start_date_ms, now_ms);
            Ok(metrics)
        }

        #[ink(message)]
        pub fn metrics_for_period(&self, app_id: AccountId, start_date_ms: u64, now_ms: u64) -> MetricValue {
            // The start date may be several month away. When did the current period start?
            let now_days = now_ms / MS_PER_DAY;
            let start_days = start_date_ms / MS_PER_DAY;
            let period_elapsed_days = (now_days - start_days) % 31;
            let period_start_days = now_days - period_elapsed_days;

            let mut month_metrics = MetricValue::default();

            for day in period_start_days..=now_days {
                let day_of_month = day % 31;
                let day_key = MetricKey { app_id, day_of_month };
                if let Some(day_metrics) = self.metrics.get(&day_key) {
                    month_metrics.add_assign(day_metrics);
                }
            }
            month_metrics
        }

        #[ink(message)]
        pub fn report_metrics(
            &mut self,
            app_id: AccountId,
            day_start_ms: u64,
            stored_bytes: u128,
            requests: u128,
        ) -> Result<()> {
            let reporter = self.env().caller();
            self.only_reporter(&reporter)?;

            enforce_time_is_start_of_day(day_start_ms)?;
            let day = day_start_ms / MS_PER_DAY;
            let day_of_month = day % 31;

            let key = MetricKey { app_id, day_of_month };
            let metrics = MetricValue { stored_bytes, requests };

            /* TODO(Aurel): support starting a new month, and enable this block.
            // If key exists, take the maximum of each metric value.
            let mut metrics = metrics;
            if let Some(previous) = self.metrics.get(&key) {
                metrics.max_assign(previous);
            }
            */

            self.metrics.insert(key.clone(), metrics.clone());

            self.env().emit_event(NewMetric {
                reporter,
                key,
                metrics,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn finalize_metric_period(&mut self, start_ms: u64) -> Result<()> {
            let reporter = self.env().caller();
            self.only_reporter(&reporter)?;

            enforce_time_is_start_of_day(start_ms)?;
            self.current_period_ms = start_ms + MS_PER_DAY;

            self.env().emit_event(MetricPeriodFinalized {
                reporter,
                start_ms,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get_current_period_ms(&self) -> u64 {
            self.current_period_ms
        }

        #[ink(message)]
        pub fn is_within_limit(&self, app_id: AccountId) -> bool {
            let metrics: MetricValue = self.metrics_since_subscription(app_id).unwrap();
            let current_tier_limit = self.tier_limit_of(app_id);
            if metrics.requests > current_tier_limit[0] || metrics.stored_bytes > current_tier_limit[1] {
                return false;
            }

            true
        }
    }


    // ---- Utils ----
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        OnlyOwner,
        OnlyReporter,
        SameDepositValue,
        NoPermission,
        InsufficientDeposit,
        TransferFailed,
        ZeroBalance,
        OverLimit,
        TidOutOfBound,
        ContractPaused,
        ContractActive,
        UnexpectedTimestamp,
        NoSubscription,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    const MS_PER_DAY: u64 = 24 * 3600 * 1000;

    fn enforce_time_is_start_of_day(ms: u64) -> Result<()> {
        if ms % MS_PER_DAY == 0 {
            Ok(())
        } else {
            Err(Error::UnexpectedTimestamp)
        }
    }


    #[cfg(test)]
    mod tests {
        use ink_env::{call, test, DefaultEnvironment, test::{default_accounts, recorded_events}};
        use ink_lang as ink;
        use scale::Decode;

        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <Ddc as ::ink_lang::BaseEvent>::Type;

        fn make_contract() -> Ddc {
            Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string())
        }

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

            let mut subscription = contract.subscriptions.get(&payer).unwrap();

            assert_eq!(subscription.end_date_ms, 31 * MS_PER_DAY);
            assert_eq!(subscription.balance, 500);

            contract.subscribe(3).unwrap();

            subscription = contract.subscriptions.get(&payer).unwrap();

            assert_eq!(subscription.end_date_ms, 31 * MS_PER_DAY * 2);
            assert_eq!(subscription.balance, 1000);

            // assert_eq!(contract.balance_of(payer), 2);
        }

        /*
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
        */

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

            // Endownment equivalence. Inititalize SC address with balance 1000
            let contract_id = ink_env::test::get_current_contract_account_id::<ink_env::DefaultEnvironment>();
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(contract_id.unwrap(), 1000);

            assert_eq!(contract.subscribe(3), Ok(()));
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), true);
            assert_eq!(contract.balance_of_contract(), 1000);
            assert_eq!(contract.transfer_all_balance(AccountId::from([0x0; 32])), Ok(()));
            assert_eq!(contract.balance_of_contract(), 0);
        }

        /// Sets the caller
        fn set_caller(caller: AccountId) {
            let callee =
                ink_env::account_id::<ink_env::DefaultEnvironment>().unwrap_or([0x0; 32].into());
            test::push_execution_context::<Environment>(
                caller,
                callee,
                1000000,
                1000000,
                test::CallData::new(call::Selector::new([0x00; 4])), // dummy
            );
        }

        #[ink::test]
        fn report_metrics_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let reporter_id = accounts.alice;
            let app_id = accounts.charlie;

            let metrics = MetricValue { stored_bytes: 11, requests: 12 };
            let big_metrics = MetricValue { stored_bytes: 100, requests: 300 };
            let double_big_metrics = MetricValue { stored_bytes: 200, requests: 600 };
            let some_day = 9999;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day; // Midnight time on some day.
            let today_key = MetricKey { app_id, day_of_month: some_day % 31 };

            let yesterday_ms = (some_day - 1) * ms_per_day; // Midnight time on some day.
            let yesterday_key = MetricKey { app_id, day_of_month: (some_day - 1) % 31 };

            let next_month_ms = (some_day + 31) * ms_per_day; // Midnight time on some day.
            let next_month_key = MetricKey { app_id, day_of_month: (some_day + 31) % 31 };

            // Unauthorized report, we are not a reporter.
            let err = contract.report_metrics(app_id, 0, metrics.stored_bytes, metrics.requests);
            assert_eq!(err, Err(Error::OnlyReporter));

            // No metric yet.
            assert_eq!(contract.metrics.get(&today_key), None);
            assert_eq!(contract.metrics_for_period(app_id, 0, today_ms), MetricValue::default());

            // Authorize our admin account to be a reporter too.
            contract.add_reporter(reporter_id).unwrap();

            // Wrong day format.
            let err = contract.report_metrics(app_id, today_ms + 1, metrics.stored_bytes, metrics.requests);
            assert_eq!(err, Err(Error::UnexpectedTimestamp));

            // Store metrics.
            contract.report_metrics(app_id, yesterday_ms, big_metrics.stored_bytes, big_metrics.requests).unwrap();
            contract.report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests).unwrap();
            assert_eq!(contract.metrics.get(&yesterday_key), Some(&big_metrics));
            assert_eq!(contract.metrics.get(&today_key), Some(&metrics));

            // Update with bigger metrics.
            contract.report_metrics(app_id, today_ms, big_metrics.stored_bytes, big_metrics.requests).unwrap();
            assert_eq!(contract.metrics.get(&today_key), Some(&big_metrics));

            // The metrics for the month is yesterday + today, both big_metrics now.
            assert_eq!(contract.metrics_for_period(app_id, 0, today_ms), double_big_metrics);
            assert_eq!(contract.metrics_for_period(app_id, yesterday_ms, today_ms), double_big_metrics);

            // If the app start date was today, then its metrics would be only today.
            assert_eq!(contract.metrics_for_period(app_id, today_ms, today_ms), big_metrics);

            // Update one month later, overwriting the same day slot.
            assert_eq!(contract.metrics.get(&next_month_key), Some(&big_metrics));
            contract.report_metrics(app_id, next_month_ms, metrics.stored_bytes, metrics.requests).unwrap();
            assert_eq!(contract.metrics.get(&next_month_key), Some(&metrics));

            // Some other account has no metrics.
            let other_key = MetricKey { app_id: accounts.bob, day_of_month: 0 };
            assert_eq!(contract.metrics.get(&other_key), None);
        }

        #[ink::test]
        fn metrics_since_subscription_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.charlie;

            // No subscription yet.
            assert_eq!(contract.metrics_since_subscription(app_id), Err(Error::NoSubscription));

            // Charlie subscribes for her app. The start date will be 0.
            set_caller(app_id);
            contract.subscribe(1).unwrap();
            test::pop_execution_context(); // Back to Alice admin.

            // Subscription without metrics.
            assert_eq!(contract.metrics_since_subscription(app_id), Ok(MetricValue { stored_bytes: 0, requests: 0 }));

            // Subscription with metrics.
            contract.add_reporter(accounts.alice).unwrap();
            contract.report_metrics(app_id, 0, 12, 34).unwrap();
            assert_eq!(contract.metrics_since_subscription(app_id), Ok(MetricValue { stored_bytes: 12, requests: 34 }));
        }

        #[ink::test]
        fn finalize_metric_period_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let yesterday_ms = 9999 * MS_PER_DAY; // Midnight time on some day.
            let today_ms = yesterday_ms + MS_PER_DAY;

            // Unauthorized report, we are not a reporter.
            let err = contract.finalize_metric_period(yesterday_ms);
            assert_eq!(err, Err(Error::OnlyReporter));

            // Authorize our admin account to be a reporter too.
            contract.add_reporter(accounts.alice).unwrap();

            // Wrong day format.
            let err = contract.finalize_metric_period(yesterday_ms + 1);
            assert_eq!(err, Err(Error::UnexpectedTimestamp));

            // Finalize today.
            contract.finalize_metric_period(yesterday_ms).unwrap();
            assert_eq!(contract.get_current_period_ms(), today_ms);
        }

        // ---- Admin: Reporters ----
        #[ink::test]
        fn add_and_remove_reporters_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800, "DDC".to_string());

            let new_reporter = AccountId::from([0x1; 32]);

            assert!(!contract.is_reporter(new_reporter));
            contract.add_reporter(new_reporter).unwrap();
            assert!(contract.is_reporter(new_reporter));
            contract.remove_reporter(new_reporter).unwrap();
            assert!(!contract.is_reporter(new_reporter));

            let raw_events = recorded_events().collect::<Vec<_>>();
            assert_eq!(2, raw_events.len());

            if let Event::ReporterAdded(ReporterAdded { reporter }) = <Event as Decode>::decode(&mut &raw_events[0].data[..]).unwrap() {
                assert_eq!(reporter, new_reporter);
            } else {
                panic!("Wrong event type");
            }

            if let Event::ReporterRemoved(ReporterRemoved { reporter }) = <Event as Decode>::decode(&mut &raw_events[1].data[..]).unwrap() {
                assert_eq!(reporter, new_reporter);
            } else {
                panic!("Wrong event type");
            }
        }

        #[ink::test]
        fn is_within_limit_works_outside_limit() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.alice;
            let metrics = MetricValue { stored_bytes: 99999, requests: 10 };

            let some_day = 0;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day;

            contract.subscribe(1).unwrap();

            assert_eq!(contract.is_within_limit(app_id), true);

            contract.add_reporter(accounts.alice).unwrap();
            contract.report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests).unwrap();

            assert_eq!(contract.is_within_limit(app_id), false)
        }

        #[ink::test]
        fn is_within_limit_works_within_limit() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.alice;
            let metrics = MetricValue { stored_bytes: 5, requests: 10 };
            let some_day = 9999;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day;

            contract.subscribe(1).unwrap();

            contract.add_reporter(accounts.alice).unwrap();
            contract.report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests).unwrap();

            assert_eq!(contract.is_within_limit(app_id), true)
        }
    }
}
