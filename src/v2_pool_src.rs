use std::sync::Arc;

use ethers::{abi::Address, contract::Contract, types::{H160, U256}};
use ethers_providers::Provider;
use tokio::sync::RwLock;

use crate::{
    err::PoolUpdateError, mult_provider::MultiProvider, trade::Trade, token::Token
};

#[derive(Debug)]
pub struct V2PoolSrc {
    pub address: Address,
    pub token0: Arc<RwLock<Token>>,
    pub token1: Arc<RwLock<Token>>,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub reserves0: U256,
    pub reserves1: U256,
    pub contract: Contract<Provider<MultiProvider>>,
}

impl V2PoolSrc {
    // Private constructor
    async fn new(
        exchange: String,
        version: String,
        fee: u32,
        address: Address,
        token0: Arc<RwLock<Token>>,
        token1: Arc<RwLock<Token>>,
        contract: Contract<Provider<MultiProvider>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange,
            version,
            fee,
            reserves0: U256::from(0),
            reserves1: U256::from(0),
            contract,
        }
    }

    pub async fn new_with_update(
        exchange: String,
        version: String,
        fee: u32,
        address: Address,
        token0: Arc<RwLock<Token>>,
        token1: Arc<RwLock<Token>>,
        contract: Contract<Provider<MultiProvider>>,
    ) -> Self {
        let mut instance =
            V2PoolSrc::new(exchange, version, fee, address, token0, token1, contract).await;
        instance.update().await;
        instance
    }

    pub async fn update(&mut self) -> Result<(), PoolUpdateError> {
        let reserves_call_result = self
            .contract
            .method::<(), (U256, U256, U256)>("getReserves", ());
        match reserves_call_result {
            Ok(reserves) => {
                let var = reserves.call_raw().await;
                println!("reserves {:?}", var);
                match var {
                    Ok((reserve0, reserve1, time)) => {
                        self.reserves0 = reserve0;
                        self.reserves1 = reserve1;
                        Ok(())
                    }
                    Err(erro) => {
                        println!("contract call error {}", erro);
                        return Err(PoolUpdateError::from(erro));
                    }
                }
            }
            Err(erro) => {
                println!("abi erro {}", erro);
                return Err(PoolUpdateError::from(erro));
            }
        }
    }
}
