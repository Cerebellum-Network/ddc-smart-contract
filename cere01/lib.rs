#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod enterprise_assets {
    #[cfg(not(feature = "ink-as-dependency"))]
    const DS_LIMIT: usize = 8;

    #[ink(storage)]
    pub struct EnterpriseAssets {
        /// Smart Contract Owner Account.
        sc_owner: AccountId,
        /// The total supply.
        total_supply: Balance,
        /// The balance of each user.
        balances: ink_storage::collections::HashMap<AccountId, Balance>,
        /// List of distribution accounts
        ds_list: [AccountId; DS_LIMIT],
        /// Number of distribution accounts
        number_of_ds: u8,
        /// User list with time limit
        time_limit_list: ink_storage::collections::HashMap<AccountId, u64>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct ErrorDS {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct IssueRestrctiveAsset {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        time_limit: u64,
    }

    impl EnterpriseAssets {
        #[ink(constructor)]
        pub fn new(initial_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let mut balances = ink_storage::collections::HashMap::new();
            let time_limit_list = ink_storage::collections::HashMap::new();

            let ds_list_temp = [caller; DS_LIMIT];

            balances.insert(caller, initial_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });

            Self {
                sc_owner: caller,
                total_supply: initial_supply,
                balances,
                ds_list: ds_list_temp,
                number_of_ds: 1,
                time_limit_list,
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_or_zero(&owner)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance, transaction_fee: Balance) -> bool {
            self.transfer_from_to(self.env().caller(), to, value, transaction_fee)
        }

        #[ink(message)]
        pub fn get_distribution_accounts(&self) -> [AccountId; DS_LIMIT] {
            self.ds_list
        }

        #[ink(message)]
        pub fn add_distribution_account(&mut self, ds_address: AccountId) -> bool {
            let caller = self.env().caller();
            let saved_sc_owner = self.sc_owner;

            if caller != saved_sc_owner {
                return false;
            }

            let mut current_ds_list: [AccountId; DS_LIMIT] = self.ds_list;
            let number_of_ds_variable: u8 = self.number_of_ds;
            current_ds_list[usize::from(number_of_ds_variable)] = ds_address;
            self.ds_list = current_ds_list;
            self.number_of_ds = number_of_ds_variable + 1;
            true
        }

        #[ink(message)]
        pub fn get_issue_restrictive_asset(&self, user_address: AccountId) -> u64 {
            *self.time_limit_list.get(&user_address).unwrap_or(&0)
        }

        #[ink(message)]
        pub fn issue_restricted_asset(
            &mut self,
            user_address: AccountId,
            value: Balance,
            has_time_limit: bool,
            time_limit: u64,
            transaction_fee: Balance,
        ) -> bool {
            let caller = self.env().caller();

            if has_time_limit {
                self.time_limit_list.insert(user_address, time_limit);
                self.env().emit_event(IssueRestrctiveAsset {
                    from: Some(caller),
                    to: Some(user_address),
                    time_limit: time_limit,
                });

                self.transfer_from_to(caller, user_address, value, transaction_fee);
                return true;
            }
            false
        }

        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, value: Balance, transaction_fee: Balance) -> bool {
            let ds_account_list = self.get_distribution_accounts();
            let is_from_ds : bool = ds_account_list.contains(&from);
            let is_to_ds : bool = ds_account_list.contains(&to);

            if is_from_ds || is_to_ds {
                let from_balance = self.balance_of_or_zero(&from);
                if from_balance < value {
                    return false;
                }
    
                // Refund the fee from SC account to the caller
                let _refund = self.env().transfer(from, transaction_fee);

                // Update the sender's balance.
                self.balances.insert(from, from_balance - value);
    
                // Update the receiver's balance.
                let to_balance = self.balance_of_or_zero(&to);
                self.balances.insert(to, to_balance + value);
    
                self.env().emit_event(Transfer {
                    from: Some(from),
                    to: Some(to),
                    value,
                });
               return true;
            }
            
            self.env().emit_event(ErrorDS {
                from: Some(from),
                to: Some(to),
                value,
            });
            false
        }

        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_env::{
            call,
            test,
        };
        use ink_lang as ink;

        #[ink::test]
        fn new_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let enterprise_assets = create_contract(contract_balance, total_supply);
            assert_eq!(enterprise_assets.total_supply(), total_supply);
        }

        #[ink::test]
        fn balance_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let enterprise_assets = create_contract(contract_balance, total_supply);
            assert_eq!(enterprise_assets.total_supply(), total_supply);
            assert_eq!(enterprise_assets.balance_of(accounts.alice), total_supply);
            assert_eq!(enterprise_assets.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let mut enterprise_assets = create_contract(contract_balance, total_supply);

            assert_eq!(enterprise_assets.balance_of(accounts.eve), 0);
            assert_eq!(enterprise_assets.transfer(accounts.eve, 100, 10), true);
            assert_eq!(enterprise_assets.balance_of(accounts.eve), 100);

            // Add eve to distribution accounts.
            assert!(enterprise_assets.add_distribution_account(accounts.eve), true);

            // set sender
            set_sender(accounts.eve);
            // set balance 
            set_balance(accounts.eve, 0);

            assert_eq!(enterprise_assets.transfer(AccountId::from([0x04; 32]), 50, 10), true);
            assert_eq!(enterprise_assets.balance_of(AccountId::from([0x04; 32])), 50);
            assert_eq!(get_balance(accounts.eve), 10)
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let mut enterprise_assets = create_contract(contract_balance, total_supply);

            // set sender
            set_sender(accounts.eve);
            // set balance 
            set_balance(accounts.eve, 0);
            assert_eq!(enterprise_assets.transfer(accounts.bob, 10, 10), false);
            assert_eq!(get_balance(accounts.eve), 0)
        }

        #[ink::test]
        fn get_distribution_accounts_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let enterprise_assets = create_contract(contract_balance, total_supply);

            let ds_account_list = enterprise_assets.get_distribution_accounts();
            assert_eq!(ds_account_list.len(), DS_LIMIT);
            assert_eq!(ds_account_list[0], accounts.alice);
        }

        #[ink::test]
        pub fn add_distribution_account_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let mut enterprise_assets = create_contract(contract_balance, total_supply);

            let ds_account_list = enterprise_assets.get_distribution_accounts();
            assert!(enterprise_assets.add_distribution_account(accounts.bob), true);
            assert_eq!(ds_account_list.len(), DS_LIMIT);
            assert_eq!(enterprise_assets.number_of_ds, 2);
        }

        #[ink::test]
        fn get_restrictive_asset_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let enterprise_assets = create_contract(contract_balance, total_supply);

            let time_limit = enterprise_assets.get_issue_restrictive_asset(accounts.alice);
            assert_eq!(time_limit, 0);
        }

        #[ink::test]
        pub fn issue_restrictive_asset_works() {
            let contract_balance = 100;
            let total_supply = 1000;
            let accounts = default_accounts();
            let mut enterprise_assets = create_contract(contract_balance, total_supply);
           
            assert!(enterprise_assets.issue_restricted_asset(accounts.bob, 100, true, 1000, 10), true);
            assert_eq!(enterprise_assets.get_issue_restrictive_asset(accounts.bob), 1000);
            assert_eq!(enterprise_assets.balance_of(accounts.bob), 100);
        }

        /// Creates a new instance of `GiveMe` with `initial_balance`.
        ///
        /// Returns the `contract_instance`.
        fn create_contract(initial_balance: Balance, total_supply: Balance) -> EnterpriseAssets {
            let accounts = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            EnterpriseAssets::new(total_supply)
        }
        
        /// Returns the `contract address`.
        fn contract_id() -> AccountId {
            ink_env::test::get_current_contract_account_id::<ink_env::DefaultEnvironment>(
            )
            .expect("Cannot get contract id")
        }

        /// Sets the callee
        fn set_sender(sender: AccountId) {
            let callee = ink_env::account_id::<ink_env::DefaultEnvironment>()
                .unwrap_or([0x0; 32].into());
            test::push_execution_context::<Environment>(
                sender,
                callee,
                1000000,
                1000000,
                test::CallData::new(call::Selector::new([0x00; 4])), // dummy
            );
        }

        /// Returns the default accounts
        fn default_accounts(
        ) -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("Off-chain environment should have been initialized already")
        }

        /// Sets the contract account
        fn set_balance(account_id: AccountId, balance: Balance) {
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(
                account_id, balance,
            )
            .expect("Cannot set account balance");
        }

        /// Returns the balance
        fn get_balance(account_id: AccountId) -> Balance {
            ink_env::test::get_account_balance::<ink_env::DefaultEnvironment>(account_id)
                .expect("Cannot set account balance")
        }
    }
}
