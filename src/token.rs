use std::clone;

use ethers::{contract::Contract, providers::{Provider, Ws}, types::H160};

#[derive(Clone,Debug)]
pub struct TokenData{
    pub name: String,
    pub address: H160,
    pub symbol: String,
    pub contract: Contract<Provider<Ws>>,
}

impl TokenData{
        pub fn new(
            name:String,
            address: H160,
            symbol: String,
            contract: Contract<Provider<Ws>>,
        ) -> Self {
            TokenData {name,address,symbol,contract}
        }
    }
