use std::str::FromStr;
use crate::pool_utils::TradeData;
use bigdecimal::{self, BigDecimal, FromPrimitive};
use ethers::{
    abi::Address,
    contract::Contract,
    providers::{Provider, Ws},
    types::{H160, U256},
};

pub trait Pool {
    async fn update(&mut self);
    fn trade(&self, amount_in: u32, from: bool) -> TradeData;
}

#[derive(Debug)]
pub struct V2Pool {
    pub address: Address,
    pub token0: H160,
    pub token1: H160,
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
        token0: H160,
        token1: H160,
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

    
    pub async fn new_with_update(
        address: Address,
        token0: H160,
        token1: H160,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = V2Pool::new(address, token0, token1, contract).await;
        instance.update().await;
        instance
    }

}

impl Pool for V2Pool {
    
    async fn update(&mut self) {
        let reserves_call_result = self
            .contract
            .method::<(), (U256, U256, U256)>("getReserves", ());
        match reserves_call_result {
            Ok(Okay) => {
                let var = Okay.call_raw().await;

                match var {
                    Ok((reserve0, reserve1, time)) => {
                        self.reserves0 = reserve0;
                        self.reserves1 = reserve1;
                    }
                    Err(erro) => println!("contract call error {}", erro),
                }
            }
            Err(erro) => println!("abi erro {}", erro),
        }
    }

    fn trade(&self, amount_in: u32, from0: bool) -> TradeData {
        let big_amount_in = BigDecimal::from_u32(amount_in).unwrap();
        let r0 = BigDecimal::from_str(&self.reserves0.to_string()).unwrap();
        let r1 = BigDecimal::from_str(&self.reserves1.to_string()).unwrap();
            
        // Calculate the input and output reserves based on the trade direction
        let (reserve_in, reserve_out) = if from0 { (&r0, &r1) } else { (&r1, &r0) };

        // Fee multiplier: assume self.fee is in basis points (e.g., 30 for 0.3%)
        let fee_multiplier =
            BigDecimal::from(1) - (BigDecimal::from(self.fee) / BigDecimal::from(10_000));

        // Adjust the input amount for the fee
        let amount_in_with_fee = &big_amount_in * &fee_multiplier;

        // Calculate amount out using the constant product formula
        // amount_out = (reserve_out * amount_in_with_fee) / (reserve_in + amount_in_with_fee)
        let numerator = reserve_out * &amount_in_with_fee;
        let denominator = reserve_in + &amount_in_with_fee;
        let amount_out = numerator / denominator;

        // New reserves after the swap
        let (new_r0, new_r1) = if from0 {
            (&r0 + &big_amount_in, &r1 - &amount_out)
        } else {
            (&r0 - &amount_out, &r1 + &big_amount_in)
        };

        // Calculate current and new prices
        let current_price = if from0 { &r1 / &r0 } else { &r0 / &r1 };
        let new_price = if from0 {
            &new_r1 / &new_r0
        } else {
            &new_r0 / &new_r1
        };

        // Price impact
        let price_impact = (&current_price - &new_price) / &current_price;

        // Create the trade data object
        TradeData {
            from0,
            amount_in: big_amount_in.clone(),
            amount_out,
            price_impact,
            fee: &big_amount_in - &amount_in_with_fee, // Fee amount
            raw_price: current_price,
        }
    }
}

#[derive(Debug)]
pub struct V3Pool {
    pub address: Address,
    pub token0: H160,
    pub token1: H160,
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
        token0: H160,
        token1: H160,
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

    pub async fn new_with_update(
        address: Address,
        token0: H160,
        token1: H160,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = Self::new(address, token0, token1, contract).await;
        instance.update().await;
        instance
    }


}

impl Pool for V3Pool {
    async fn update(&mut self) {}
 
    fn trade(&self, amount_in: u32, from: bool) -> TradeData {
        
        TradeData {
            from0: from,
            amount_in: BigDecimal::from_str("0").unwrap(),
            amount_out: BigDecimal::from_str("0").unwrap(),
            price_impact: BigDecimal::from_str("0").unwrap(),
            fee: BigDecimal::from_str("0").unwrap(),
            raw_price: BigDecimal::from_str("0").unwrap(),
        }
    }
}
