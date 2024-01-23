#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod markets {

    use trait_erc20::{TERC20};
    use erc721::Erc721Ref;

    #[ink(storage)]
    pub struct Markets {
        acceptable_erc20: ink::contract_ref!(TERC20),
        erc721: Erc721Ref,
        price: Balance,
        minted_count: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        ERC20TransferFailed,
        ERC721MintFailed,
        ERC721TransferFailed,
    }
    pub type Result<T> = core::result::Result<T, Error>;


    impl Markets {
        #[ink(constructor)]
        pub fn new(erc721: Erc721Ref, erc20: AccountId) -> Self {
            Self {
                acceptable_erc20: erc20.into(),
                erc721,
                price: 20,
                minted_count: 0,
            }
        }

        #[ink(message)]
        pub fn buy_nft(&mut self) -> Result<()> {
            let sender = self.env().caller();
            let _res = self.acceptable_erc20.transfer_from(sender, self.env().account_id(), self.price);
            if _res.is_err() {
                return Err(Error::ERC20TransferFailed);
            }
            self.minted_count += 1;
            let mint_res = self.erc721.mint(self.minted_count);
            if mint_res.is_err() {
                return Err(Error::ERC721MintFailed);
            }

            let transfer_res = self.erc721.transfer(sender, self.minted_count);
            if transfer_res.is_err() {
                return Err(Error::ERC721TransferFailed);
            }

            Ok(())
        }

        #[ink(message)]
        pub fn minted_count(&self) -> u32 {
            self.minted_count
        }
    }
}
