#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod ddc {
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        lazy::Lazy,
        traits::{PackedLayout, SpreadLayout},
    };
    use scale::{Decode, Encode};

    // ---- Storage ----
    #[ink(storage)]
    pub struct Ddc {
        // -- Admin --
        /// Owner of Contract.
        owner: Lazy<AccountId>,
        pause: bool,

        // -- Tiers --
        /// HashMap of tier_id: vector of [tier_id, tier_fee, tier_throughput_limit, tier_storage_limit]
        service: StorageHashMap<u128, Vec<u128>>,

        // -- App Subscriptions --
        /// Mapping from owner to number of owned coins.
        balances: StorageHashMap<AccountId, Balance>,
        subscriptions: StorageHashMap<AccountId, AppSubscription>,

        // -- Admin: Reporters --
        reporters: StorageHashMap<AccountId, ()>,

        // -- DDC Nodes --
        ddc_nodes: StorageHashMap<String, DDCNode>,

        // -- Metrics Reporting --
        pub metrics: StorageHashMap<MetricKey, MetricValue>,
        current_period_ms: u64,

        pub metrics_ddn: StorageHashMap<MetricKeyDDN, MetricValue>,
    }

    impl Ddc {
        /// Constructor that initializes the contract
        /// Give tier3fee, tier3limit, tier2fee, tier2limit, tier1fee, and tier1 limit to initialize
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
        ) -> Self {
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

            let now: u64 = Self::env().block_timestamp(); // Epoch in milisecond
            let today_ms = now - now % MS_PER_DAY; // Beginning of deploy date in Epoch milisecond

            let instance = Self {
                owner: Lazy::new(caller),
                service: service_v,
                balances: StorageHashMap::new(),
                subscriptions: StorageHashMap::new(),
                reporters: StorageHashMap::new(),
                ddc_nodes: StorageHashMap::new(),
                metrics: StorageHashMap::new(),
                metrics_ddn: StorageHashMap::new(),
                current_period_ms: today_ms,
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

        /// As owner, withdraw tokens to the given account. The destination account can be the same
        /// as the contract owner. Some balance must be left in the contract as subsistence deposit.
        #[ink(message)]
        pub fn withdraw(&mut self, destination: AccountId, amount: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            if destination == AccountId::default() {
                return Err(Error::InvalidAccount);
            }

            // Check that the amount requested is *strictly* less than the contract balance.
            // If it is exactly the same, it is probably an error because then the contract
            // will not have any deposit left for its subsistence.
            if self.env().balance() <= amount {
                return Err(Error::InsufficientBalance);
            }

            match self.env().transfer(destination, amount) {
                Err(_e) => Err(Error::TransferFailed),
                Ok(_v) => Ok(()),
            }
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
        pub fn change_tier_limit(
            &mut self,
            tier_id: u128,
            new_throughput_limit: u128,
            new_storage_limit: u128,
        ) -> Result<()> {
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

    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
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
            if service_v[1] > fee_value {
                //TODO: We probably need to summarize the existing balance with provided, in case app wants to deposit more than monthly amount
                return Err(Error::InsufficientDeposit);
            }

            let subscription_opt = self.subscriptions.get(&payer);
            let now = Self::env().block_timestamp();
            let mut subscription: AppSubscription;

            if subscription_opt.is_none() || subscription_opt.unwrap().end_date_ms < now {
                subscription = AppSubscription {
                    start_date_ms: now,
                    end_date_ms: now + 31 * MS_PER_DAY,
                    tier_id,
                    balance: value,
                };
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

    // ---- DDC nodes ----
    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct DDCNode {
        p2p_id: String,
        url: String,
    }

    #[ink(event)]
    pub struct DDCNodeAdded {
        #[ink(topic)]
        p2p_id: String,
        url: String,
    }

    #[ink(event)]
    pub struct DDCNodeRemoved {
        #[ink(topic)]
        p2p_id: String,
    }

    impl Ddc {
        /// Return the list of all DDC nodes
        #[ink(message)]
        pub fn get_all_ddc_nodes(&self) -> Vec<DDCNode> {
            self.ddc_nodes.values().cloned().collect()
        }

        /// Add DDC node to the list
        #[ink(message)]
        pub fn add_ddc_node(&mut self, p2p_id: String, url: String) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            self.ddc_nodes.insert(
                p2p_id.clone(),
                DDCNode {
                    p2p_id: p2p_id.clone(),
                    url: url.clone(),
                },
            );
            Self::env().emit_event(DDCNodeAdded { p2p_id, url });

            Ok(())
        }

        /// Check if DDC node is in the list
        #[ink(message)]
        pub fn is_ddc_node(&self, p2p_id: String) -> bool {
            self.ddc_nodes.contains_key(&p2p_id)
        }

        /// Removes DDC node from the list
        #[ink(message)]
        pub fn remove_ddc_node(&mut self, p2p_id: String) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            self.ddc_nodes.take(&p2p_id);
            Self::env().emit_event(DDCNodeRemoved { p2p_id });

            Ok(())
        }
    }

    // ---- Metrics Reporting ----
    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricKey {
        reporter: AccountId,
        app_id: AccountId,
        day_of_month: u64,
    }

    // ---- Metric per DDN ----
    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricKeyDDN {
        ddn_id: Vec<u8>,
        day_of_month: u64,
    }

    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
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
    pub struct NewMetricDDN {
        #[ink(topic)]
        reporter: AccountId,
        #[ink(topic)]
        key: MetricKeyDDN,
        metrics: MetricValue,
    }

    #[ink(event)]
    pub struct MetricPeriodFinalized {
        #[ink(topic)]
        reporter: AccountId,
        start_ms: u64,
    }

    /// Get median value from a vector
    fn get_median<T: Clone + Ord>(source: Vec<T>) -> Option<T> {
        let length = source.len();
        let mut sorted_source = source;
        // sort_unstable is faster, it doesn't preserve the order of equal elements
        sorted_source.sort_unstable();
        let index_correction = length != 0 && length % 2 == 0;
        let median_index = length / 2 - index_correction as usize;
        sorted_source.get(median_index).cloned()
    }

    impl Ddc {
        #[ink(message)]
        pub fn metrics_since_subscription(&self, app_id: AccountId) -> Result<MetricValue> {
            let subscription = self
                .subscriptions
                .get(&app_id)
                .ok_or(Error::NoSubscription)?;

            let now_ms = Self::env().block_timestamp() as u64;
            let metrics = self.metrics_for_period(app_id, subscription.start_date_ms, now_ms);
            Ok(metrics)
        }

        #[ink(message)]
        pub fn metrics_for_period(
            &self,
            app_id: AccountId,
            start_date_ms: u64,
            now_ms: u64,
        ) -> MetricValue {
            // The start date may be several month away. When did the current period start?
            let now_days = now_ms / MS_PER_DAY;
            let start_days = start_date_ms / MS_PER_DAY;
            let period_elapsed_days = (now_days - start_days) % 31;
            let period_start_days = now_days - period_elapsed_days;

            let mut month_metrics = MetricValue::default();

            for day in period_start_days..=now_days {
                let day_of_month = day % 31;

                let mut day_stored_bytes: Vec<u128> = Vec::new();
                let mut day_reqests: Vec<u128> = Vec::new();

                for reporter in self.reporters.keys() {
                    let reporter_day_key = MetricKey {
                        reporter: reporter.clone(),
                        app_id,
                        day_of_month,
                    };

                    if let Some(reporter_day_metric) = self.metrics.get(&reporter_day_key) {
                        day_stored_bytes.push(reporter_day_metric.stored_bytes);
                        day_reqests.push(reporter_day_metric.requests);
                    }
                }

                month_metrics.add_assign(&MetricValue {
                    stored_bytes: get_median(day_stored_bytes).unwrap_or(0),
                    requests: get_median(day_reqests).unwrap_or(0),
                });
            }

            month_metrics
        }

        #[ink(message)]
        pub fn metrics_for_ddn(&self, ddn_id: Vec<u8>) -> Vec<MetricValue> {
            let mut month_metrics: Vec<MetricValue> = Vec::new();

            for day_of_month in 0..31 {
                let day_key = MetricKeyDDN {
                    ddn_id: ddn_id.clone(),
                    day_of_month,
                };
                let mut item = MetricValue {
                    stored_bytes: 0,
                    requests: 0,
                };

                if let Some(value) = self.metrics_ddn.get(&day_key) {
                    item = value.clone();
                }

                month_metrics.push(item.clone());
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

            let key = MetricKey {
                reporter,
                app_id,
                day_of_month,
            };
            let metrics = MetricValue {
                stored_bytes,
                requests,
            };

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
        pub fn report_metrics_ddn(
            &mut self,
            ddn_id: Vec<u8>,
            day_start_ms: u64,
            stored_bytes: u128,
            requests: u128,
        ) -> Result<()> {
            let reporter = self.env().caller();
            self.only_reporter(&reporter)?;

            enforce_time_is_start_of_day(day_start_ms)?;
            let day = day_start_ms / MS_PER_DAY;
            let day_of_month = day % 31;

            let key = MetricKeyDDN {
                ddn_id,
                day_of_month,
            };
            let metrics = MetricValue {
                stored_bytes,
                requests,
            };

            self.metrics_ddn.insert(key.clone(), metrics.clone());

            self.env().emit_event(NewMetricDDN {
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

            self.env()
                .emit_event(MetricPeriodFinalized { reporter, start_ms });

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
            if metrics.requests > current_tier_limit[0]
                || metrics.stored_bytes > current_tier_limit[1]
            {
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
        InsufficientBalance,
        InvalidAccount,
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
        use crate::ddc::Error::*;
        use ink_env::{
            call, test,
            test::{default_accounts, recorded_events},
            AccountId, DefaultEnvironment,
        };
        use ink_lang as ink;

        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <Ddc as ::ink_lang::BaseEvent>::Type;

        fn make_contract() -> Ddc {
            Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800)
        }

        /// We test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            let contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            assert_eq!(contract.tier_deposit(1), 8);
            assert_eq!(contract.tier_deposit(2), 4);
            assert_eq!(contract.tier_deposit(3), 2);
        }

        /// Test if a function can only be called by the contract admin
        #[ink::test]
        fn onlyowner_works() {
            let contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
        }

        /// Test that we can transfer owner to another account
        #[ink::test]
        fn transfer_ownership_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            contract
                .transfer_ownership(AccountId::from([0x0; 32]))
                .unwrap();
            assert_eq!(contract.only_owner(AccountId::from([0x0; 32])), Ok(()));
        }

        /// Test the contract can take payment from users
        #[ink::test]
        fn subscribe_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
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

        /// Test the total balance of the contract is correct
        #[ink::test]
        fn balance_of_contract_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            let payer_one = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer_one), 0);
            assert_eq!(contract.subscribe(3), Ok(()));
            assert_eq!(contract.balance_of_contract(), 0);
        }

        /// Test the contract can return the correct tier if given an account id
        #[ink::test]
        fn tier_id_of_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            let payer_one = AccountId::from([0x1; 32]);
            assert_eq!(contract.balance_of(payer_one), 0);
            assert_eq!(contract.subscribe(2), Ok(()));
            assert_eq!(contract.tier_id_of(payer_one), 2);
        }

        /// Test we can read metrics
        #[ink::test]
        fn get_all_tiers_works() {
            let contract = Ddc::new(2000, 2000, 2000, 4000, 4000, 4000, 8000, 8000, 8000);

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
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
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
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            assert_eq!(contract.change_tier_limit(3, 100, 100), Ok(()));
            assert_eq!(contract.change_tier_limit(2, 200, 200), Ok(()));
            assert_eq!(contract.change_tier_limit(1, 300, 300), Ok(()));
            assert_eq!(contract.get_tier_limit(3), vec![100, 100]);
            assert_eq!(contract.get_tier_limit(2), vec![200, 200]);
            assert_eq!(contract.get_tier_limit(1), vec![300, 300]);
        }

        /// Test the contract owner can flip the status of the contract
        /// Can pause and unpause the contract
        #[ink::test]
        fn flip_contract_status_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            assert_eq!(contract.paused_or_not(), false);
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), true);
            assert_eq!(contract.flip_contract_status(), Ok(()));
            assert_eq!(contract.paused_or_not(), false);
        }

        /// Test the contract owner can transfer all the balance out of the contract after it is paused
        #[ink::test]
        fn withdraw_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();

            // Endownment equivalence. Inititalize SC address with balance 1000
            set_balance(contract_id(), 1000);
            set_balance(accounts.bob, 0);
            assert_eq!(balance_of(contract_id()), 1000);

            // Non-owner cannot withdraw.
            set_caller(accounts.bob);
            assert_eq!(contract.withdraw(accounts.bob, 200), Err(OnlyOwner));
            assert_eq!(balance_of(contract_id()), 1000);
            undo_set_caller(); // Back to Alice owner.

            // Cannot withdraw to the zero account by mistake.
            assert_eq!(
                contract.withdraw(AccountId::default(), 200),
                Err(InvalidAccount)
            );

            // Cannot withdraw the entire balance by mistake.
            assert_eq!(
                contract.withdraw(accounts.bob, 1000),
                Err(InsufficientBalance)
            );

            // Can withdraw some tokens.
            assert_eq!(contract.withdraw(accounts.bob, 200), Ok(()));
            assert_eq!(balance_of(accounts.bob), 200);
            assert_eq!(balance_of(contract_id()), 800);
            assert_eq!(contract.balance_of_contract(), 800);
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

        fn undo_set_caller() {
            test::pop_execution_context();
        }

        fn balance_of(account: AccountId) -> Balance {
            test::get_account_balance::<ink_env::DefaultEnvironment>(account).unwrap()
        }

        fn set_balance(account: AccountId, balance: Balance) {
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(account, balance)
                .unwrap();
        }

        fn contract_id() -> AccountId {
            ink_env::test::get_current_contract_account_id::<ink_env::DefaultEnvironment>().unwrap()
        }

        #[ink::test]
        fn get_median_works() {
            let vec = vec![7, 1, 7, 9999, 9, 7, 0];
            assert_eq!(get_median(vec), Some(7));
        }

        #[ink::test]
        fn report_metrics_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let reporter_id = accounts.alice;
            let app_id = accounts.charlie;

            let metrics = MetricValue {
                stored_bytes: 11,
                requests: 12,
            };
            let big_metrics = MetricValue {
                stored_bytes: 100,
                requests: 300,
            };
            let double_big_metrics = MetricValue {
                stored_bytes: 200,
                requests: 600,
            };
            let some_day = 9999;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day; // Midnight time on some day.
            let today_key = MetricKey {
                reporter: reporter_id,
                app_id,
                day_of_month: some_day % 31,
            };

            let yesterday_ms = (some_day - 1) * ms_per_day; // Midnight time on some day.
            let yesterday_key = MetricKey {
                reporter: reporter_id,
                app_id,
                day_of_month: (some_day - 1) % 31,
            };

            let next_month_ms = (some_day + 31) * ms_per_day; // Midnight time on some day.
            let next_month_key = MetricKey {
                reporter: reporter_id,
                app_id,
                day_of_month: (some_day + 31) % 31,
            };

            // Unauthorized report, we are not a reporter.
            let err = contract.report_metrics(app_id, 0, metrics.stored_bytes, metrics.requests);
            assert_eq!(err, Err(Error::OnlyReporter));

            // No metric yet.
            assert_eq!(contract.metrics.get(&today_key), None);
            assert_eq!(
                contract.metrics_for_period(app_id, 0, today_ms),
                MetricValue::default()
            );

            // Authorize our admin account to be a reporter too.
            contract.add_reporter(reporter_id).unwrap();

            // Wrong day format.
            let err = contract.report_metrics(
                app_id,
                today_ms + 1,
                metrics.stored_bytes,
                metrics.requests,
            );
            assert_eq!(err, Err(Error::UnexpectedTimestamp));

            // Store metrics.
            contract
                .report_metrics(
                    app_id,
                    yesterday_ms,
                    big_metrics.stored_bytes,
                    big_metrics.requests,
                )
                .unwrap();
            contract
                .report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests)
                .unwrap();
            assert_eq!(contract.metrics.get(&yesterday_key), Some(&big_metrics));
            assert_eq!(contract.metrics.get(&today_key), Some(&metrics));

            // Update with bigger metrics.
            contract
                .report_metrics(
                    app_id,
                    today_ms,
                    big_metrics.stored_bytes,
                    big_metrics.requests,
                )
                .unwrap();
            assert_eq!(contract.metrics.get(&today_key), Some(&big_metrics));

            // The metrics for the month is yesterday + today, both big_metrics now.
            assert_eq!(
                contract.metrics_for_period(app_id, 0, today_ms),
                double_big_metrics
            );
            assert_eq!(
                contract.metrics_for_period(app_id, yesterday_ms, today_ms),
                double_big_metrics
            );

            // If the app start date was today, then its metrics would be only today.
            assert_eq!(
                contract.metrics_for_period(app_id, today_ms, today_ms),
                big_metrics
            );

            // Update one month later, overwriting the same day slot.
            assert_eq!(contract.metrics.get(&next_month_key), Some(&big_metrics));
            contract
                .report_metrics(
                    app_id,
                    next_month_ms,
                    metrics.stored_bytes,
                    metrics.requests,
                )
                .unwrap();
            assert_eq!(contract.metrics.get(&next_month_key), Some(&metrics));

            // Some other account has no metrics.
            let other_key = MetricKey {
                reporter: reporter_id,
                app_id: accounts.bob,
                day_of_month: 0,
            };
            assert_eq!(contract.metrics.get(&other_key), None);
        }

        #[ink::test]
        fn median_works() {
            let mut contract = make_contract();

            let alice = AccountId::from([0x01; 32]);
            let bob = AccountId::from([0x02; 32]);
            let charlie = AccountId::from([0x03; 32]);
            let django = AccountId::from([0x04; 32]);
            let eve = AccountId::from([0x05; 32]);
            let frank = AccountId::from([0x06; 32]);

            contract.add_reporter(alice).unwrap();
            contract.add_reporter(bob).unwrap();
            contract.add_reporter(charlie).unwrap();
            contract.add_reporter(django).unwrap();
            contract.add_reporter(eve).unwrap();
            contract.add_reporter(frank).unwrap();

            let day1 = 10001;
            let day1_ms = day1 * MS_PER_DAY;
            let day2 = 10002;
            let day2_ms = day2 * MS_PER_DAY;
            let day3 = 10003;
            let day3_ms = day3 * MS_PER_DAY;
            let day4 = 10004;
            let day4_ms = day4 * MS_PER_DAY;
            let day5 = 10005;
            let day5_ms = day5 * MS_PER_DAY;

            let day1_alice_django_key = MetricKey {
                reporter: alice,
                app_id: django,
                day_of_month: day1 % 31,
            };

            // No metric yet.
            assert_eq!(contract.metrics.get(&day1_alice_django_key), None);
            assert_eq!(
                contract.metrics_for_period(django, 0, day5_ms),
                MetricValue::default()
            );

            // bob day1: [0, 6, 8, 8, 100] -> 8
            // bob day2: [2, 4, 4, 5, 6] -> 4
            // bob day3: [5, 8, 10, 11, 11] -> 10
            // bob day4: [8, 16, 20, 50, 80] -> 20
            // bob day5: [0, 0, 2, 2, 2] -> 2

            // charlie day1: [0, 1, 4, 5, 5] -> 4
            // charlie day2: [2, 4, 4, 5, 5] -> 4
            // charlie day3: [2, 2, 2, 11, 11] -> 2
            // charlie day4: [0, 4, 5, 5, 5] -> 5
            // charlie day5: [0, 0, 10, 11, 11]-> 10

            // django day1: [1, 1, 1, 1, 5] -> 1
            // django day2: [0, 5, 5, 5, 5] -> 5
            // django day3: [1, 8, 8, 8, 1000] -> 8
            // django day4: [2, 2, 10, 10] -> 2 ?
            // django day5: [2, 2, 2, 10] -> 2

            // eve day1: [5, 5, 5, 5] -> 5
            // eve day2: [1, 5, 5, 5] -> 5
            // eve day3: [1, 6, 6, 10] -> 6
            // eve day4: [2, 4, 6, 10] -> 4
            // eve day5: [1, 1, 1, 100] -> 1

            // frank day1: [7, 7, 7] -> 7
            // frank day2: [0, 10, 10] -> 10
            // frank day3: [2, 2, 10] -> 2
            // frank day4: [0, 10, 20] -> 10
            // frank day5: [1, 2, 3] -> 2

            // alice day1: [2, 5] -> 2
            // alice day2: [0, 10] -> 0
            // alice day3: [7, 7] -> 7
            // alice day4: [2] - 2
            // alice day5: [] - 0

            // Day 1
            set_caller(bob);
            contract.report_metrics(bob, day1_ms, 8, 1).unwrap();
            contract.report_metrics(charlie, day1_ms, 0, 2).unwrap();
            contract.report_metrics(django, day1_ms, 1, 3).unwrap();
            contract.report_metrics(eve, day1_ms, 5, 4).unwrap();
            contract.report_metrics(frank, day1_ms, 7, 5).unwrap();
            contract.report_metrics(alice, day1_ms, 2, 6).unwrap();
            undo_set_caller();

            set_caller(charlie);
            contract.report_metrics(bob, day1_ms, 6, 1).unwrap();
            contract.report_metrics(charlie, day1_ms, 1, 2).unwrap();
            contract.report_metrics(django, day1_ms, 1, 3).unwrap();
            contract.report_metrics(eve, day1_ms, 5, 4).unwrap();
            undo_set_caller();

            set_caller(django);
            contract.report_metrics(bob, day1_ms, 8, 1).unwrap();
            contract.report_metrics(charlie, day1_ms, 4, 2).unwrap();
            contract.report_metrics(django, day1_ms, 5, 3).unwrap();
            contract.report_metrics(eve, day1_ms, 5, 4).unwrap();
            contract.report_metrics(frank, day1_ms, 7, 5).unwrap();
            contract.report_metrics(alice, day1_ms, 5, 6).unwrap();
            undo_set_caller();

            set_caller(eve);
            contract.report_metrics(bob, day1_ms, 0, 1).unwrap();
            contract.report_metrics(charlie, day1_ms, 5, 2).unwrap();
            contract.report_metrics(django, day1_ms, 1, 3).unwrap();
            contract.report_metrics(eve, day1_ms, 5, 4).unwrap();
            contract.report_metrics(frank, day1_ms, 7, 5).unwrap();

            undo_set_caller();

            set_caller(frank);
            contract.report_metrics(bob, day1_ms, 100, 1).unwrap();
            contract.report_metrics(charlie, day1_ms, 5, 2).unwrap();
            contract.report_metrics(django, day1_ms, 1, 3).unwrap();
            undo_set_caller();

            // Day 2
            set_caller(bob);
            contract.report_metrics(bob, day2_ms, 2, 1).unwrap();
            contract.report_metrics(charlie, day2_ms, 5, 2).unwrap();
            contract.report_metrics(django, day2_ms, 5, 3).unwrap();
            contract.report_metrics(eve, day2_ms, 5, 4).unwrap();
            contract.report_metrics(frank, day2_ms, 0, 5).unwrap();
            contract.report_metrics(alice, day2_ms, 0, 6).unwrap();
            undo_set_caller();

            set_caller(charlie);
            contract.report_metrics(bob, day2_ms, 4, 1).unwrap();
            contract.report_metrics(charlie, day2_ms, 5, 2).unwrap();
            contract.report_metrics(django, day2_ms, 0, 3).unwrap();
            contract.report_metrics(eve, day2_ms, 1, 4).unwrap();
            contract.report_metrics(frank, day2_ms, 10, 5).unwrap();
            undo_set_caller();

            set_caller(django);
            contract.report_metrics(bob, day2_ms, 5, 1).unwrap();
            contract.report_metrics(charlie, day2_ms, 4, 2).unwrap();
            contract.report_metrics(django, day2_ms, 5, 3).unwrap();
            contract.report_metrics(eve, day2_ms, 5, 4).unwrap();
            contract.report_metrics(frank, day2_ms, 10, 5).unwrap();
            contract.report_metrics(alice, day2_ms, 10, 6).unwrap();
            undo_set_caller();

            set_caller(eve);
            contract.report_metrics(bob, day2_ms, 6, 1).unwrap();
            contract.report_metrics(charlie, day2_ms, 4, 2).unwrap();
            contract.report_metrics(django, day2_ms, 5, 3).unwrap();
            contract.report_metrics(eve, day2_ms, 5, 4).unwrap();
            undo_set_caller();

            set_caller(frank);
            contract.report_metrics(bob, day2_ms, 4, 1).unwrap();
            contract.report_metrics(charlie, day2_ms, 2, 2).unwrap();
            contract.report_metrics(django, day2_ms, 5, 3).unwrap();
            undo_set_caller();

            // Day3
            set_caller(bob);
            contract.report_metrics(bob, day3_ms, 11, 1).unwrap();
            contract.report_metrics(charlie, day3_ms, 11, 2).unwrap();
            contract.report_metrics(django, day3_ms, 1000, 3).unwrap();
            contract.report_metrics(eve, day3_ms, 1, 4).unwrap();
            contract.report_metrics(frank, day3_ms, 10, 5).unwrap();
            contract.report_metrics(alice, day3_ms, 7, 6).unwrap();
            undo_set_caller();

            set_caller(charlie);
            contract.report_metrics(bob, day3_ms, 11, 1).unwrap();
            contract.report_metrics(charlie, day3_ms, 2, 2).unwrap();
            contract.report_metrics(django, day3_ms, 8, 3).unwrap();
            contract.report_metrics(eve, day3_ms, 6, 4).unwrap();
            undo_set_caller();

            set_caller(django);
            contract.report_metrics(bob, day3_ms, 8, 1).unwrap();
            contract.report_metrics(charlie, day3_ms, 11, 2).unwrap();
            contract.report_metrics(django, day3_ms, 8, 3).unwrap();
            contract.report_metrics(eve, day3_ms, 6, 4).unwrap();
            contract.report_metrics(frank, day3_ms, 2, 5).unwrap();
            contract.report_metrics(alice, day3_ms, 7, 6).unwrap();
            undo_set_caller();

            set_caller(eve);
            contract.report_metrics(bob, day3_ms, 10, 1).unwrap();
            contract.report_metrics(charlie, day3_ms, 2, 2).unwrap();
            contract.report_metrics(django, day3_ms, 8, 3).unwrap();
            contract.report_metrics(frank, day3_ms, 2, 5).unwrap();
            undo_set_caller();

            set_caller(frank);
            contract.report_metrics(bob, day3_ms, 5, 1).unwrap();
            contract.report_metrics(charlie, day3_ms, 2, 2).unwrap();
            contract.report_metrics(django, day3_ms, 1, 3).unwrap();
            contract.report_metrics(eve, day3_ms, 10, 4).unwrap();
            undo_set_caller();

            // Day 4
            set_caller(bob);
            contract.report_metrics(bob, day4_ms, 80, 1).unwrap();
            contract.report_metrics(charlie, day4_ms, 5, 2).unwrap();
            contract.report_metrics(django, day4_ms, 10, 3).unwrap();
            contract.report_metrics(frank, day4_ms, 20, 5).unwrap();
            contract.report_metrics(alice, day4_ms, 2, 6).unwrap();
            undo_set_caller();

            set_caller(charlie);
            contract.report_metrics(bob, day4_ms, 20, 1).unwrap();
            contract.report_metrics(charlie, day4_ms, 0, 2).unwrap();
            contract.report_metrics(django, day4_ms, 2, 3).unwrap();
            contract.report_metrics(eve, day4_ms, 2, 4).unwrap();
            contract.report_metrics(frank, day4_ms, 10, 5).unwrap();
            undo_set_caller();

            set_caller(django);
            contract.report_metrics(bob, day4_ms, 50, 1).unwrap();
            contract.report_metrics(charlie, day4_ms, 5, 2).unwrap();
            contract.report_metrics(django, day4_ms, 10, 3).unwrap();
            contract.report_metrics(eve, day4_ms, 4, 4).unwrap();
            contract.report_metrics(frank, day4_ms, 0, 5).unwrap();
            undo_set_caller();

            set_caller(eve);
            contract.report_metrics(bob, day4_ms, 8, 1).unwrap();
            contract.report_metrics(charlie, day4_ms, 5, 2).unwrap();
            contract.report_metrics(django, day4_ms, 2, 3).unwrap();
            contract.report_metrics(eve, day4_ms, 6, 4).unwrap();
            undo_set_caller();

            set_caller(frank);
            contract.report_metrics(bob, day4_ms, 16, 1).unwrap();
            contract.report_metrics(charlie, day4_ms, 4, 2).unwrap();
            contract.report_metrics(eve, day4_ms, 10, 4).unwrap();
            undo_set_caller();

            // Day 5
            set_caller(bob);
            contract.report_metrics(bob, day5_ms, 2, 1).unwrap();
            contract.report_metrics(charlie, day5_ms, 11, 2).unwrap();
            contract.report_metrics(django, day5_ms, 10, 3).unwrap();
            contract.report_metrics(eve, day5_ms, 1, 4).unwrap();
            contract.report_metrics(frank, day5_ms, 1, 5).unwrap();
            undo_set_caller();

            set_caller(charlie);
            contract.report_metrics(bob, day5_ms, 0, 1).unwrap();
            contract.report_metrics(charlie, day5_ms, 10, 2).unwrap();
            contract.report_metrics(django, day5_ms, 2, 3).unwrap();
            contract.report_metrics(frank, day5_ms, 2, 5).unwrap();
            undo_set_caller();

            set_caller(django);
            contract.report_metrics(bob, day5_ms, 0, 1).unwrap();
            contract.report_metrics(charlie, day5_ms, 11, 2).unwrap();
            contract.report_metrics(django, day5_ms, 2, 3).unwrap();
            contract.report_metrics(eve, day5_ms, 100, 4).unwrap();
            contract.report_metrics(frank, day5_ms, 3, 5).unwrap();
            undo_set_caller();

            set_caller(eve);
            contract.report_metrics(bob, day5_ms, 2, 1).unwrap();
            contract.report_metrics(charlie, day5_ms, 0, 2).unwrap();
            contract.report_metrics(django, day5_ms, 2, 3).unwrap();
            contract.report_metrics(eve, day5_ms, 1, 4).unwrap();
            undo_set_caller();

            set_caller(frank);
            contract.report_metrics(bob, day5_ms, 2, 1).unwrap();
            contract.report_metrics(charlie, day5_ms, 0, 2).unwrap();
            contract.report_metrics(eve, day5_ms, 1, 4).unwrap();
            undo_set_caller();

            // Bob
            assert_eq!(
                contract.metrics_for_period(bob, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 8,
                    requests: 1,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 4,
                    requests: 1,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 1,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 20,
                    requests: 1,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 1,
                }
            );

            assert_eq!(
                contract.metrics_for_period(bob, 0, day5_ms),
                MetricValue {
                    stored_bytes: 44,
                    requests: 5,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 12,
                    requests: 2,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 22,
                    requests: 3,
                }
            );
            assert_eq!(
                contract.metrics_for_period(bob, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 36,
                    requests: 4,
                }
            );

            // Charlie
            assert_eq!(
                contract.metrics_for_period(charlie, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 4,
                    requests: 2,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 4,
                    requests: 2,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 2,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 5,
                    requests: 2,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 2,
                }
            );

            assert_eq!(
                contract.metrics_for_period(charlie, 0, day5_ms),
                MetricValue {
                    stored_bytes: 25,
                    requests: 10,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 8,
                    requests: 4,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 6,
                }
            );
            assert_eq!(
                contract.metrics_for_period(charlie, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 21,
                    requests: 8,
                }
            );

            // Django
            assert_eq!(
                contract.metrics_for_period(django, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 1,
                    requests: 3,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 5,
                    requests: 3,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 8,
                    requests: 3,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 3,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 3,
                }
            );

            assert_eq!(
                contract.metrics_for_period(django, 0, day5_ms),
                MetricValue {
                    stored_bytes: 18,
                    requests: 15,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 6,
                    requests: 6,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 14,
                    requests: 9,
                }
            );
            assert_eq!(
                contract.metrics_for_period(django, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 17,
                    requests: 12,
                }
            );

            // Eve
            assert_eq!(
                contract.metrics_for_period(eve, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 5,
                    requests: 4,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 5,
                    requests: 4,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 6,
                    requests: 4,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 4,
                    requests: 4,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 1,
                    requests: 4,
                }
            );

            assert_eq!(
                contract.metrics_for_period(eve, 0, day5_ms),
                MetricValue {
                    stored_bytes: 21,
                    requests: 20,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 8,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 16,
                    requests: 12,
                }
            );
            assert_eq!(
                contract.metrics_for_period(eve, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 16,
                    requests: 16,
                }
            );

            // Frank
            assert_eq!(
                contract.metrics_for_period(frank, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 7,
                    requests: 5,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 5,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 5,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 10,
                    requests: 5,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 5,
                }
            );

            assert_eq!(
                contract.metrics_for_period(frank, 0, day5_ms),
                MetricValue {
                    stored_bytes: 31,
                    requests: 25,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 17,
                    requests: 10,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 19,
                    requests: 15,
                }
            );
            assert_eq!(
                contract.metrics_for_period(frank, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 24,
                    requests: 20,
                }
            );

            // Alice
            assert_eq!(
                contract.metrics_for_period(alice, day1_ms, day1_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 6,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day2_ms, day2_ms),
                MetricValue {
                    stored_bytes: 0,
                    requests: 6,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day3_ms, day3_ms),
                MetricValue {
                    stored_bytes: 7,
                    requests: 6,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day4_ms, day4_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 6,
                }
            );
            // no metrics
            assert_eq!(
                contract.metrics_for_period(alice, day5_ms, day5_ms),
                MetricValue {
                    stored_bytes: 0,
                    requests: 0,
                }
            );

            assert_eq!(
                contract.metrics_for_period(alice, 0, day5_ms),
                MetricValue {
                    stored_bytes: 11,
                    requests: 24,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day1_ms, day2_ms),
                MetricValue {
                    stored_bytes: 2,
                    requests: 12,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day1_ms, day3_ms),
                MetricValue {
                    stored_bytes: 9,
                    requests: 18,
                }
            );
            assert_eq!(
                contract.metrics_for_period(alice, day2_ms, day5_ms),
                MetricValue {
                    stored_bytes: 9,
                    requests: 18,
                }
            );
        }

        #[ink::test]
        fn metrics_since_subscription_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.charlie;

            // No subscription yet.
            assert_eq!(
                contract.metrics_since_subscription(app_id),
                Err(Error::NoSubscription)
            );

            // Charlie subscribes for her app. The start date will be 0.
            set_caller(app_id);
            contract.subscribe(1).unwrap();
            undo_set_caller(); // Back to Alice admin.

            // Subscription without metrics.
            assert_eq!(
                contract.metrics_since_subscription(app_id),
                Ok(MetricValue {
                    stored_bytes: 0,
                    requests: 0
                })
            );

            // Subscription with metrics.
            contract.add_reporter(accounts.alice).unwrap();
            contract.report_metrics(app_id, 0, 12, 34).unwrap();
            assert_eq!(
                contract.metrics_since_subscription(app_id),
                Ok(MetricValue {
                    stored_bytes: 12,
                    requests: 34
                })
            );
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

        fn decode_event(event: &ink_env::test::EmittedEvent) -> Event {
            <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer")
        }

        // ---- Admin: Reporters ----
        #[ink::test]
        fn add_and_remove_reporters_works() {
            let mut contract = Ddc::new(2, 2000, 2000, 4, 4000, 4000, 8, 8000, 800);

            let new_reporter = AccountId::from([0x1; 32]);

            assert!(!contract.is_reporter(new_reporter));
            contract.add_reporter(new_reporter).unwrap();
            assert!(contract.is_reporter(new_reporter));
            contract.remove_reporter(new_reporter).unwrap();
            assert!(!contract.is_reporter(new_reporter));

            let raw_events = recorded_events().collect::<Vec<_>>();
            assert_eq!(2, raw_events.len());

            if let Event::ReporterAdded(ReporterAdded { reporter }) = decode_event(&raw_events[0]) {
                assert_eq!(reporter, new_reporter);
            } else {
                panic!("Wrong event type");
            }

            if let Event::ReporterRemoved(ReporterRemoved { reporter }) =
                decode_event(&raw_events[1])
            {
                assert_eq!(reporter, new_reporter);
            } else {
                panic!("Wrong event type");
            }
        }

        // ---- DDC Nodes ----
        #[ink::test]
        fn get_all_ddc_nodes_works() {
            let contract = make_contract();

            // Return an empty list
            assert_eq!(contract.get_all_ddc_nodes(), vec![]);
        }

        #[ink::test]
        fn add_ddc_node_only_owner_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let p2p_id = String::from("test_p2p_id");
            let url = String::from("ws://localhost:9944");

            // Should be an owner
            set_caller(accounts.charlie);
            assert_eq!(contract.add_ddc_node(p2p_id, url), Err(Error::OnlyOwner));
        }

        #[ink::test]
        fn add_ddc_node_works() {
            let mut contract = make_contract();
            let p2p_id = String::from("test_p2p_id");
            let url = String::from("ws://localhost:9944");

            // Add DDC node
            contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

            // Should be in the list
            assert_eq!(
                contract.get_all_ddc_nodes(),
                vec![DDCNode {
                    p2p_id: p2p_id.clone(),
                    url: url.clone()
                },]
            );

            // Should emit event
            let raw_events = recorded_events().collect::<Vec<_>>();
            assert_eq!(1, raw_events.len());
            if let Event::DDCNodeAdded(DDCNodeAdded {
                p2p_id: event_p2p_id,
                url: event_url,
            }) = decode_event(&raw_events[0])
            {
                assert_eq!(event_p2p_id, p2p_id);
                assert_eq!(event_url, url);
            } else {
                panic!("Wrong event type")
            }
        }

        #[ink::test]
        fn is_ddc_node_works() {
            let mut contract = make_contract();
            let p2p_id = String::from("test_p2p_id");
            let url = String::from("ws://localhost:9944");

            // Return false if not added
            assert_eq!(contract.is_ddc_node(p2p_id.clone()), false);

            // Add DDC node
            contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

            // Should be in the list
            assert_eq!(contract.is_ddc_node(p2p_id), true);
        }

        #[ink::test]
        fn remove_ddc_node_only_owner_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let p2p_id = String::from("test_p2p_id");

            // Should be an owner
            set_caller(accounts.charlie);
            assert_eq!(contract.remove_ddc_node(p2p_id), Err(Error::OnlyOwner));
        }

        #[ink::test]
        fn remove_ddc_node_works() {
            let mut contract = make_contract();
            let p2p_id = String::from("test_p2p_id");
            let url = String::from("ws://localhost:9944");

            // Add DDC node
            contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

            // Remove DDC node
            contract.remove_ddc_node(p2p_id.clone()).unwrap();

            // Should be removed from the list
            assert_eq!(contract.get_all_ddc_nodes(), vec![]);

            // Should emit event
            let raw_events = recorded_events().collect::<Vec<_>>();
            assert_eq!(2, raw_events.len());
            if let Event::DDCNodeRemoved(DDCNodeRemoved {
                p2p_id: event_p2p_id,
            }) = decode_event(&raw_events[1])
            {
                assert_eq!(event_p2p_id, p2p_id);
            } else {
                panic!("Wrong event type")
            }
        }

        // ---- Metrics Reporting ----
        #[ink::test]
        fn is_within_limit_works_outside_limit() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.alice;
            let metrics = MetricValue {
                stored_bytes: 99999,
                requests: 10,
            };

            let some_day = 0;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day;

            contract.subscribe(1).unwrap();

            assert_eq!(contract.is_within_limit(app_id), true);

            contract.add_reporter(accounts.alice).unwrap();
            contract
                .report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests)
                .unwrap();

            assert_eq!(contract.is_within_limit(app_id), false)
        }

        #[ink::test]
        fn is_within_limit_works_within_limit() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();
            let app_id = accounts.alice;
            let metrics = MetricValue {
                stored_bytes: 5,
                requests: 10,
            };
            let some_day = 9999;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day;

            contract.subscribe(1).unwrap();

            contract.add_reporter(accounts.alice).unwrap();
            contract
                .report_metrics(app_id, today_ms, metrics.stored_bytes, metrics.requests)
                .unwrap();

            assert_eq!(contract.is_within_limit(app_id), true)
        }

        #[ink::test]
        fn report_metrics_ddn_works() {
            let mut contract = make_contract();
            let accounts = default_accounts::<DefaultEnvironment>().unwrap();

            let some_day = 9999;
            let ms_per_day = 24 * 3600 * 1000;

            let today_ms = some_day * ms_per_day;
            let ddn_id = b"12D3KooWPfi9EtgoZHFnHh1at85mdZJtj7L8n94g6LFk6e8EEk2b".to_vec();
            let stored_bytes = 99;
            let requests = 999;

            contract.add_reporter(accounts.alice).unwrap();
            contract
                .report_metrics_ddn(ddn_id.clone(), today_ms, stored_bytes, requests)
                .unwrap();

            let result = contract.metrics_for_ddn(ddn_id);

            let mut expected = vec![
                MetricValue {
                    stored_bytes: 0,
                    requests: 0,
                };
                31
            ];

            expected[17] = MetricValue {
                stored_bytes,
                requests,
            };

            assert_eq!(result, expected);
        }
    }
}
