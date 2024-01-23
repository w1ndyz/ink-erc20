#![cfg_attr(not(feature = "std"), no_std, no_main)]


#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;
    use trait_erc20::{TERC20, Result, Error};

    #[ink(storage)]
    #[derive(Default)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    impl Erc20 {
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            balances.insert(Self::env().caller(), &total_supply);
            Self {
                total_supply,
                balances,
                ..Default::default()
            }
        }



        pub fn transfer_helper(&mut self, from: &AccountId, to: &AccountId, value: Balance) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to = self.balance_of(*to);

            if balance_from < value {
                return Err(Error::BalanceTooLow);
            }

            self.balances.insert(from, &(balance_from - value));
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                value,
            });

            Ok(())
        }
    }

    impl TERC20 for Erc20 {
        #[ink(message)]
        fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).unwrap_or_default()
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            return self.transfer_helper(&sender, &to, value)
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            let allowance = self.allowances.get(&(from, sender)).unwrap_or_default();
            if allowance < value {
                return Err(Error::AllowanceTooLow);
            }
            self.allowances.insert(&(from, sender), &(allowance - value));
            return self.transfer_helper(&from, &to, value)
        }

        #[ink(message)]
        fn approve(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.allowances.insert(&(sender, to), &value);

            self.env().emit_event(Approval {
                from: Some(sender),
                to: Some(to),
                value,
            });

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <Erc20 as ink::reflect::ContractEventBase>::Type;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn constructor_works() {
            let mut erc20 = Erc20::new(1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // transfer event
            let _ = erc20.transfer(accounts.bob, 1000);
            assert_eq!(erc20.total_supply(), 1000);
            assert_eq!(erc20.balance_of(accounts.alice), 0);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emitted_events[0];
            let decoded = <Event as scale::Decode>::decode(&mut &event.data[..]).expect("failed to decode event");

            match decoded {
                Event::Transfer(Transfer{ from, to, value }) => {
                    assert_eq!(from, Some(accounts.alice));
                    assert_eq!(to, Some(accounts.bob));
                    assert_eq!(value, 1000);
                },
                _ => panic!("No transfer event emitted!"),
            }
        }

        #[ink::test]
        fn transfer_should_work() {
            let mut erc20 = Erc20::new(1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let res = erc20.transfer(accounts.bob, 100);

            assert_eq!(res, Ok(()));
            assert_eq!(erc20.balance_of(accounts.alice), 900);
            assert_eq!(erc20.balance_of(accounts.bob), 100);
        }

        #[ink::test]
        fn transfer_should_fail() {
            let mut erc20 = Erc20::new(1000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let res = erc20.transfer(accounts.bob, 1001);

            assert_eq!(res, Err(Error::BalanceTooLow));
            assert_eq!(erc20.balance_of(accounts.alice), 1000);
            assert_eq!(erc20.balance_of(accounts.bob), 0);
        }
    }


    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn e2e_transfer(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 1000;
            let constructor = Erc20Ref::new(total_supply);
            let construct_acc_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = build_message::<Erc20Ref>(construct_acc_id.clone())
                .call(|erc20| erc20.transfer(bob_acc.clone(), 100));

            let res = client
                .call(&ink_e2e::alice(), transfer_msg, 0, None)
                .await;

            assert!(res.is_ok());

            let balance_msg = build_message::<Erc20Ref>(construct_acc_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc.clone()));

            let balance_of_alice = client
                .call_dry_run(&ink_e2e::alice(), &balance_msg, 0, None)
                .await;

            assert_eq!(balance_of_alice.return_value(), 900);
            Ok(())
        }
    }
}
