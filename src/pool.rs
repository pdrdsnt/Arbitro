use axum::http::method;
use ethers::{abi::Address, contract::Contract, providers::{Provider, Ws}, types::U256};

use crate::{blockchain_db::Token, token::TokenData};


pub trait Pool {
     async fn update(&mut self);

    async fn new_with_update(
        address: Address,
        token0: TokenData,
        token1: TokenData,
        contract: Contract<Provider<Ws>>,
    ) -> Self;
}

#[derive(Debug)]
pub struct V2Pool {
    pub address: Address,
    pub token0: TokenData,
    pub token1: TokenData,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub reserves0: U256,
    pub reserves1: U256,
    pub contract: Contract<Provider<Ws>>,
}


impl V2Pool {
    // Private constructor
    async fn new(
        address: Address,
        token0: TokenData,
        token1: TokenData,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: " ".to_string(),
            version: "v2".to_string(),
            fee: 3000,
            reserves0: U256::from(0),
            reserves1: U256::from(0),
            contract,
        }
    }
}

impl Pool for V2Pool {
    async fn new_with_update(
        address: Address,
        token0: TokenData,
        token1: TokenData,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = V2Pool::new(address, token0, token1, contract).await;
        instance.update().await;
        instance
    }
    async fn update(&mut self) {
        let reserves_call_result = self.contract.method::<(), (U256, U256, U256)>("getReserves", ());
        match reserves_call_result {
            Ok(Okay) => {
                let var = Okay.call_raw().await;

                match var {
                    Ok((reserve0, reserve1, time)) => {
                        self.reserves0 = reserve0;
                        self.reserves1 = reserve1;
                    
                    }
                    Err(erro) => (println!("contract call error {}", erro)),
                }
            }
            Err(erro) => (println!("abi erro {}" , erro)),
        }

    }
}

#[derive(Debug)]
pub struct V3Pool {
    pub address: Address,
    pub token0: TokenData,
    pub token1: TokenData,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub liquidity: U256,
    pub contract: Contract<Provider<Ws>>,
}

impl V3Pool {
    // Private constructor
    async fn new(
        address: Address,
        token0: TokenData,
        token1: TokenData,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: " ".to_string(),
            version: "v2".to_string(),
            fee: 3000,
            liquidity: U256::from(0),
            contract,
        }
    }
}

impl Pool for V3Pool {
    
    async fn update(&mut self) {
        
    }
    async fn new_with_update(
            address: Address,
            token0: TokenData,
            token1: TokenData,
            contract: Contract<Provider<Ws>>,
        ) -> Self {
            let mut instance = Self::new(address, token0, token1, contract).await;
            instance.update().await;
            instance
    }
}
