#![cfg_attr(not(feature = "std"), no_std)]
#![feature(proc_macro_hygiene)] // for tests in a separate file

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
        service_tiers: StorageHashMap<u64, ServiceTier>,

        // -- App Subscriptions --
        /// Mapping from owner to number of owned coins.
        balances: StorageHashMap<AccountId, Balance>,
        subscriptions: StorageHashMap<AccountId, AppSubscription>,

        // -- Admin: Reporters --
        reporters: StorageHashMap<AccountId, ()>,
        current_period_ms: StorageHashMap<AccountId, u64>,

        // -- DDC Nodes --
        ddc_nodes: StorageHashMap<String, DDCNode>,

        // -- Metrics Reporting --
        pub metrics: StorageHashMap<MetricKey, MetricValue>,
        pub metrics_ddn: StorageHashMap<MetricKeyDDN, MetricValue>,
    }

    impl Ddc {
        /// Constructor that initializes the contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();

            let instance = Self {
                owner: Lazy::new(caller),
                service_tiers: StorageHashMap::new(),
                balances: StorageHashMap::new(),
                subscriptions: StorageHashMap::new(),
                reporters: StorageHashMap::new(),
                current_period_ms: StorageHashMap::new(),
                ddc_nodes: StorageHashMap::new(),
                metrics: StorageHashMap::new(),
                metrics_ddn: StorageHashMap::new(),
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

    #[derive(scale::Encode, Clone, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub struct ServiceTier {
        tier_id: u64,
        tier_fee: Balance,
        storage_bytes: u64,
        wcu: u64,
        rcu: u64,
    }

    impl ServiceTier {
        pub fn new(tier_id: u64, tier_fee: Balance, storage_bytes: u64, wcu: u64, rcu: u64) -> ServiceTier {
            ServiceTier {
                tier_id,
                tier_fee,
                storage_bytes,
                wcu,
                rcu,
            }
        }
    }

    #[ink(event)]
    pub struct TierAdded {
        tier_id: u64,
        tier_fee: Balance,
        storage_bytes: u64,
        wcu: u64,
        rcu: u64,
    }

    impl Ddc {
        fn calculate_new_tier_id(&self) -> u64 {
            let mut max = 0_u64;
            for key in self.service_tiers.keys() {
                let tier = self.service_tiers.get(key).unwrap();
                if tier.tier_id > max {
                    max = tier.tier_id;
                }
            }

            max + 1
        }

        #[ink(message)]
        pub fn add_tier(&mut self, tier_fee: Balance, storage_bytes: u64, wcu: u64, rcu: u64) -> Result<u64> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            let tier_id = self.calculate_new_tier_id();
            let tier = ServiceTier { tier_id, tier_fee, storage_bytes, wcu, rcu };
            self.service_tiers.insert(tier_id, tier);
            Self::env().emit_event(TierAdded { tier_id, tier_fee, storage_bytes, wcu, rcu });

            Ok(tier_id)
        }

        /// return the fee required
        #[ink(message)]
        pub fn tier_deposit(&self, tier_id: u64) -> Balance {
            if self.tid_in_bound(tier_id).is_err() {
                return 0 as Balance;
            }

            let v = self.service_tiers.get(&tier_id).unwrap();
            return v.tier_fee as Balance;
        }

        #[ink(message)]
        pub fn get_all_tiers(&self) -> Vec<ServiceTier> {
            self.service_tiers.values().cloned().collect()
        }

        /// check if tid is within 1, 2 ,3
        /// return ok or error
        fn tid_in_bound(&self, tier_id: u64) -> Result<()> {
            if self.service_tiers.get(&tier_id).is_none() {
                return Err(Error::TidOutOfBound);
            } else {
                Ok(())
            }
        }

        /// change the tier fee given the tier id and new fee
        /// Must be the contract admin to call this function
        #[ink(message)]
        pub fn change_tier_fee(&mut self, tier_id: u64, new_fee: Balance) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;

            self.diff_deposit(tier_id, new_fee)?;

            let mut tier = self.service_tiers.get_mut(&tier_id).unwrap();

            tier.tier_fee = new_fee;

            Ok(())
        }

        /// Change tier limit given tier id and a new limit
        /// Must be contract admin to call this function
        #[ink(message)]
        pub fn change_tier_limit(
            &mut self,
            tier_id: u64,
            new_storage_bytes_limit: u64,
            new_wcu_limit: u64,
            new_rcu_limit: u64,
        ) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let caller = self.env().caller();
            self.only_owner(caller)?;

            let mut tier = self.service_tiers.get_mut(&tier_id).unwrap();
            tier.storage_bytes = new_storage_bytes_limit;
            tier.wcu = new_wcu_limit;
            tier.rcu = new_rcu_limit;

            Ok(())
        }

        /// Check if the new fee is the same as the old fee
        /// Return error if they are the same
        fn diff_deposit(&self, tier_id: u64, new_value: Balance) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            let v = self.service_tiers.get(&tier_id).unwrap();
            if v.tier_fee as Balance != new_value {
                return Ok(());
            } else {
                return Err(Error::SameDepositValue);
            }
        }

        /// Return tier limit given a tier id
        fn get_tier_limit(&self, tier_id: u64) -> ServiceTier {
            self.tid_in_bound(tier_id).unwrap();

            self.service_tiers.get(&tier_id).unwrap().clone()
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
        tier_id: u64,

        balance: Balance,
        last_update_ms: u64, // initially creation time
    }

    #[derive(
    Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct AppSubscriptionLimit {
        storage_bytes: u64,
        wcu: u64,
        rcu: u64,
    }

    impl AppSubscriptionLimit {
        pub fn new(storage_bytes: u64, wcu: u64, rcu: u64) -> AppSubscriptionLimit {
            AppSubscriptionLimit {
                storage_bytes,
                wcu,
                rcu,
            }
        }
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
        pub fn tier_id_of(&self, acct: AccountId) -> u64 {
            let tier_id = self.get_tier_id(&acct);
            tier_id
        }

        /// Return the tier limit corresponding the account
        #[ink(message)]
        pub fn tier_limit_of(&self, acct: AccountId) -> ServiceTier {
            let tier_id = self.get_tier_id(&acct);
            let tl = self.get_tier_limit(tier_id);

            tl.clone()
        }

        /// Return tier id given an account
        fn get_tier_id(&self, owner: &AccountId) -> u64 {
            let subscription = self.subscriptions.get(owner).unwrap();
            subscription.tier_id
        }

        fn get_end_date_ms(&self, subscription: AppSubscription) -> u64 {
            let tier_id = subscription.tier_id.clone();
            let tier = self.service.get(&tier_id).unwrap();
            let price = tier[1]; // get tier fee
            let prepaid_time_ms = subscription.balance * PERIOD_MS as u128 / price;

            subscription.last_update_ms.clone() + prepaid_time_ms as u64
        }

        fn get_consumed_balance(&self, subscription: AppSubscription) -> Balance {
            let now_ms = Self::env().block_timestamp();
            let duration_consumed = now_ms - subscription.last_update_ms.clone();
            let tier_id = subscription.tier_id.clone();
            let tier = self.service.get(&tier_id).unwrap();
            let price = tier[1] / 31 / MS_PER_DAY as u128; // get tier fee

            duration_consumed as u128 * price
        }

        fn actualize_subscription(&mut self, subscription: &mut AppSubscription) {
            let now_ms = Self::env().block_timestamp();
            let consumed = self.get_consumed_balance(subscription.clone());

            if consumed > subscription.balance {
                subscription.balance = 0;
            } else {
                subscription.balance -= consumed;
            }
            subscription.last_update_ms = now_ms;
        }

        fn set_tier(&mut self, subscription: &mut AppSubscription, new_tier_id: u128) {
            self.actualize_subscription(subscription);
            subscription.tier_id = new_tier_id.clone();
        }

        #[ink(message)]
        pub fn get_app_limit(&self, app: AccountId) -> Result<AppSubscriptionLimit> {
            let now_ms = Self::env().block_timestamp() as u64;

            self.get_app_limit_at_time(app, now_ms)
        }

        pub fn get_app_limit_at_time(&self, app: AccountId, now_ms: u64) -> Result<AppSubscriptionLimit> {
            let subscription_opt = self.subscriptions.get(&app);
            if subscription_opt.is_none() {
                return Err(Error::NoSubscription);
            }
            let subscription = subscription_opt.unwrap();

            if self.tid_in_bound(subscription.tier_id).is_err() {
                return Ok(AppSubscriptionLimit::new(0, 0, 0));
            }

            let current_tier = self.service_tiers.get(&subscription.tier_id).unwrap();


            // actual
            if subscription.end_date_ms >= now_ms {
                Ok(AppSubscriptionLimit::new(
                    current_tier.storage_bytes,
                    current_tier.wcu,
                    current_tier.rcu,
                ))
            } else { // expired
                let free_tier = self.get_free_tier()?;

                Ok(AppSubscriptionLimit::new(
                    free_tier.storage_bytes,
                    free_tier.wcu,
                    free_tier.rcu,
                ))
            }
        }

        pub fn get_free_tier(&self) -> Result<ServiceTier> {
            for tier_key in self.service_tiers.keys() {
                let current_tier = self.service_tiers.get(tier_key).unwrap();
                if current_tier.tier_fee == 0 {
                    return Ok(current_tier.clone());
                }
            }

            Err(Error::NoFreeTier)
        }

        /// Receive payment from the participating DDC node
        /// Store payment into users balance map
        /// Initialize user metrics map
        #[ink(message, payable)]
        pub fn subscribe(&mut self, tier_id: u64) -> Result<()> {
            self.tid_in_bound(tier_id)?;
            self.only_active()?;
            let payer = self.env().caller();
            let value = self.env().transferred_balance();
            let fee_value = value;
            let service_v = self.service_tiers.get(&tier_id).unwrap();
            if service_v.tier_fee > fee_value {
                //TODO: We probably need to summarize the existing balance with provided, in case app wants to deposit more than monthly amount
                return Err(Error::InsufficientDeposit);
            }

            let subscription_opt = self.subscriptions.get(&payer);
            let now = Self::env().block_timestamp();
            let mut subscription: AppSubscription;

            if subscription_opt.is_none() || self.get_end_date_ms(subscription_opt.unwrap().clone()) < now {
                subscription = AppSubscription {
                    start_date_ms: now,
                    tier_id,

                    last_update_ms: now,
                    balance: value,
                };
            } else {
                subscription = subscription_opt.unwrap().clone();

                subscription.balance = subscription.balance + value;

                if subscription.tier_id != tier_id {
                    self.set_tier(&mut subscription, tier_id);
                }
            }

            self.subscriptions.insert(payer, subscription.clone());
            self.env().emit_event(Deposit {
                from: Some(payer),
                value,
            });

            return Ok(());
        }

        #[ink(message)]
        pub fn refund(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let subscription_opt = self.subscriptions.get(&caller);
            if subscription_opt.is_none() {
                return Err(Error::NoSubscription);
            }

            let mut subscription = subscription_opt.unwrap().clone();
            let consumed_balance = self.get_consumed_balance(subscription.clone()) as Balance;
            if consumed_balance > subscription.balance {
                return Ok(());
            }

            let to_refund = subscription.balance - consumed_balance;

            let result = match self.env().transfer(caller, to_refund) {
                Err(_e) => {
                    Err(Error::TransferFailed)
                },
                Ok(_v) => Ok(()),
            };

            if !result.is_ok() {
                return result;
            }

            subscription.balance = 0;
            self.actualize_subscription(&mut subscription);
            self.subscriptions.insert(caller, subscription.clone());

            Ok(())
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
        day_of_period: u64,
    }

    // ---- Metric per DDN ----
    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricKeyDDN {
        ddn_id: Vec<u8>,
        day_of_period: u64,
    }

    #[derive(
        Default, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(Debug, scale_info::TypeInfo))]
    pub struct MetricValue {
        start_ms: u64,
        storage_bytes: u64,
        wcu_used: u64,
        rcu_used: u64,
    }

    impl MetricValue {
        pub fn add_assign(&mut self, other: &Self) {
            self.storage_bytes += other.storage_bytes;
            self.wcu_used += other.wcu_used;
            self.rcu_used += other.rcu_used;
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
            subscription_start_ms: u64,
            now_ms: u64,
        ) -> MetricValue {
            // The start date may be several months away. When did the current period start?
            let (period_start_days, now_days) =
                get_current_period_days(subscription_start_ms, now_ms);

            let mut period_metrics = MetricValue {
                start_ms: period_start_days * MS_PER_DAY,
                storage_bytes: 0,
                wcu_used: 0,
                rcu_used: 0,
            };

            for day in period_start_days..=now_days {
                let mut day_storage_bytes: Vec<u64> = Vec::new();
                let mut day_wcu_used: Vec<u64> = Vec::new();
                let mut day_rcu_used: Vec<u64> = Vec::new();

                for reporter in self.reporters.keys() {
                    let reporter_day_metric = self.metrics_for_day(reporter.clone(), app_id, day);
                    if let Some(reporter_day_metric) = reporter_day_metric {
                        day_storage_bytes.push(reporter_day_metric.storage_bytes);
                        day_wcu_used.push(reporter_day_metric.wcu_used);
                        day_rcu_used.push(reporter_day_metric.rcu_used);
                    }
                }

                period_metrics.add_assign(&MetricValue {
                    storage_bytes: get_median(day_storage_bytes).unwrap_or(0),
                    wcu_used: get_median(day_wcu_used).unwrap_or(0),
                    rcu_used: get_median(day_rcu_used).unwrap_or(0),
                    start_ms: 0, // Ignored.
                });
            }
            period_metrics
        }

        fn metrics_for_day(
            &self,
            reporter: AccountId,
            app_id: AccountId,
            day: u64,
        ) -> Option<&MetricValue> {
            let day_of_period = day % PERIOD_DAYS;
            let day_key = MetricKey {
                reporter,
                app_id,
                day_of_period,
            };
            let day_metrics = self.metrics.get(&day_key);

            // Ignore out-of-date metrics from a previous period.
            if let Some(day_metrics) = day_metrics {
                if day_metrics.start_ms != day * MS_PER_DAY {
                    return None;
                }
            }

            day_metrics
        }

        #[ink(message)]
        pub fn metrics_for_ddn(&self, ddn_id: Vec<u8>) -> Vec<MetricValue> {
            let now_ms = Self::env().block_timestamp() as u64;
            self.metrics_for_ddn_at_time(ddn_id, now_ms)
        }

        pub fn metrics_for_ddn_at_time(&self, ddn_id: Vec<u8>, now_ms: u64) -> Vec<MetricValue> {
            let mut period_metrics: Vec<MetricValue> = Vec::with_capacity(PERIOD_DAYS as usize);

            let last_day = now_ms / MS_PER_DAY + 1; // non-inclusive.
            let first_day = if last_day >= PERIOD_DAYS {
                last_day - PERIOD_DAYS
            } else {
                0
            };

            for day in first_day..last_day {
                let metrics = self.metrics_for_ddn_day(ddn_id.clone(), day);
                period_metrics.push(metrics);
            }

            period_metrics
        }

        fn metrics_for_ddn_day(&self, ddn_id: Vec<u8>, day: u64) -> MetricValue {
            let day_of_period = day % PERIOD_DAYS;
            let day_key = MetricKeyDDN {
                ddn_id,
                day_of_period,
            };
            let start_ms = day * MS_PER_DAY;

            let day_metrics = self.metrics_ddn.get(&day_key);
            match day_metrics {
                // Return metrics that exists and is not outdated.
                Some(metrics) if metrics.start_ms == start_ms => metrics.clone(),

                // Otherwise, return 0 for missing or outdated metrics from a previous period.
                _ => MetricValue {
                    start_ms,
                    storage_bytes: 0,
                    wcu_used: 0,
                    rcu_used: 0,
                },
            }
        }

        #[ink(message)]
        pub fn report_metrics(
            &mut self,
            app_id: AccountId,
            day_start_ms: u64,
            storage_bytes: u64,
            wcu_used: u64,
            rcu_used: u64,
        ) -> Result<()> {
            let reporter = self.env().caller();
            self.only_reporter(&reporter)?;

            enforce_time_is_start_of_day(day_start_ms)?;
            let day = day_start_ms / MS_PER_DAY;
            let day_of_period = day % PERIOD_DAYS;

            let key = MetricKey {
                reporter,
                app_id,
                day_of_period,
            };
            let metrics = MetricValue {
                start_ms: day_start_ms,
                storage_bytes,
                wcu_used,
                rcu_used,
            };

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
            storage_bytes: u64,
            wcu_used: u64,
            rcu_used: u64,
        ) -> Result<()> {
            let reporter = self.env().caller();
            self.only_reporter(&reporter)?;

            enforce_time_is_start_of_day(day_start_ms)?;
            let day = day_start_ms / MS_PER_DAY;
            let day_of_period = day % PERIOD_DAYS;

            let key = MetricKeyDDN {
                ddn_id,
                day_of_period,
            };
            let metrics = MetricValue {
                start_ms: day_start_ms,
                storage_bytes,
                wcu_used,
                rcu_used,
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
            let next_period_ms = start_ms + MS_PER_DAY;
            self.current_period_ms.insert(reporter, next_period_ms);

            self.env()
                .emit_event(MetricPeriodFinalized { reporter, start_ms });

            Ok(())
        }

        #[ink(message)]
        pub fn get_current_period_ms(&self) -> u64 {
            let caller = self.env().caller();
            self.get_current_period_ms_of(caller)
        }

        #[ink(message)]
        pub fn get_current_period_ms_of(&self, reporter_id: AccountId) -> u64 {
            let current_period_ms = self.current_period_ms.get(&reporter_id);
            match current_period_ms {
                None => {
                    let now: u64 = Self::env().block_timestamp(); // Epoch in milisecond
                    let today_ms = now - now % MS_PER_DAY; // The beginning of today
                    today_ms
                }
                Some(current_period_ms) => *current_period_ms,
            }
        }

        #[ink(message)]
        pub fn is_within_limit(&self, app_id: AccountId) -> bool {
            let metrics = match self.metrics_since_subscription(app_id) {
                Err(_) => return false,
                Ok(metrics) => metrics,
            };
            let current_tier_limit = self.tier_limit_of(app_id);
            let bytes_ok = metrics.storage_bytes <= current_tier_limit.storage_bytes;
            let wcu_ok = metrics.storage_bytes <= current_tier_limit.wcu;
            let rcu_ok = metrics.storage_bytes <= current_tier_limit.rcu;

            bytes_ok && wcu_ok && rcu_ok
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
        NoFreeTier,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    const MS_PER_DAY: u64 = 24 * 3600 * 1000;
    const PERIOD_DAYS: u64 = 31;
    const PERIOD_MS: u64 = PERIOD_DAYS * MS_PER_DAY;

    fn get_current_period_days(subscription_start_ms: u64, now_ms: u64) -> (u64, u64) {
        let now_days = now_ms / MS_PER_DAY;
        let start_days = subscription_start_ms / MS_PER_DAY;
        let period_elapsed_days = (now_days - start_days) % PERIOD_DAYS;
        let period_start_days = now_days - period_elapsed_days;
        (period_start_days, now_days)
    }

    fn enforce_time_is_start_of_day(ms: u64) -> Result<()> {
        if ms % MS_PER_DAY == 0 {
            Ok(())
        } else {
            Err(Error::UnexpectedTimestamp)
        }
    }

    #[cfg(test)]
    mod tests;
}
