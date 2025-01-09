use std::str::FromStr;

use ethers::{abi::Address, contract::Contract, providers::{Provider, Ws}, types::U256};
use bigdecimal::{self, BigDecimal, FromPrimitive};
use crate::{pool_utils::TradeData, token::TokenData};


pub trait Pool {
    async fn update(&mut self);
    
    async fn new_with_update(
        address: Address,
        token0: TokenData,
        token1: TokenData,
        contract: Contract<Provider<Ws>>,
    ) -> Self;

    fn trade(&self, amount_in: u32,from: bool) -> TradeData;
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

    fn trade(&self, amount_in: u32,from0: bool) -> TradeData{
        let big_amount_in = BigDecimal::from_u32(amount_in).unwrap();
        let r0 = BigDecimal::from_str(&self.reserves0.to_string()).unwrap();
        let r1 = BigDecimal::from_str(&self.reserves1.to_string()).unwrap();
        let current_price= if from0 {&r1 / &r0} else { &r0 / &r1 };
        let nr0 = if from0 { &r0 + &big_amount_in } else {r0 - &big_amount_in};
        let nr1 = if from0 { &r1 + &big_amount_in } else {r1 - &big_amount_in};
        let new_price= if from0 {&nr1 / &nr0} else { &nr0 / &nr1 }; 
        let price_impact = &current_price - new_price;
        let fee = (&big_amount_in / 100) * (self.fee / 10000);
        let final_price = &current_price - &price_impact - &fee;

        let trade_data = TradeData { from0, amount_in: big_amount_in,amount_out: final_price,price_impact,fee,raw_price: current_price};
        trade_data

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


    fn trade(&self, amount_in: u32,from: bool) -> TradeData {
        let trade_data = TradeData { 
            from0: true, 
            amount_in: BigDecimal::from_str("0").unwrap(),
            amount_out: BigDecimal::from_str("0").unwrap(),
            price_impact:  BigDecimal::from_str("0").unwrap(),
            fee: BigDecimal::from_str("0").unwrap(),
            raw_price:  BigDecimal::from_str("0").unwrap(),
        };
        trade_data

    }
}
