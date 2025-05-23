use std::sync::Arc;

use ethers::{
    abi::Address,
    contract::Contract,
    types::{H160, U256},
};
use ethers_providers::Provider;
use tokio::sync::RwLock;

use crate::{
    err::PoolUpdateError, mult_provider::MultiProvider, token::Token, trade::Trade,
    v2_pool_sim::V2PoolSim,
};

#[derive(Debug)]
pub struct V2PoolSrc {
    pub address: Address,
    pub token0: Arc<RwLock<Token>>,
    pub token0_addr: H160,
    pub token1: Arc<RwLock<Token>>,
    pub token1_addr: H160,
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
        token0_addr: H160,
        token1: Arc<RwLock<Token>>,
        token1_addr: H160,
        contract: Contract<Provider<MultiProvider>>,
    ) -> Self {
        let mut instance = V2PoolSrc {
            exchange,
            version,
            fee,
            address,
            token0,
            token0_addr,
            token1,
            token1_addr,
            contract,
            reserves0: U256::zero(),
            reserves1: U256::zero(),
        };
        instance.update().await;
        instance
    }

    pub async fn into_sim(&self) -> V2PoolSim {
        let token0 = self.token0.read().await.clone();
        let token1 = self.token1.read().await.clone();
        V2PoolSim {
            address: self.address,
            token0,
            token1,
            exchange: self.exchange.clone(),
            version: self.version.clone(),
            fee: self.fee,
            reserves0: self.reserves0,
            reserves1: self.reserves1,
        }
    }

    pub async fn update(&mut self) -> Result<H160, PoolUpdateError> {
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
                        Ok(self.address)
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


    pub async fn trade(&self, amount_in: U256, from0: bool) -> Option<Trade> {
        if (from0 && self.reserves0 == U256::zero()) || (!from0 && self.reserves1 == U256::zero()) {
            return None;
        }

        // 2. Get reserves in proper decimal scale
        let (mut reserve_in, mut reserve_out) = match from0 {
            true => (self.reserves0, self.reserves1),
            false => (self.reserves1, self.reserves0),
        };

        // 3. Apply V2 fee calculation correctly (0.3% fee)
        let amount_in_less_fee = amount_in
            .checked_mul(U256::from(997))?
            .checked_div(U256::from(1000))?;
        let numerator = amount_in_less_fee.checked_mul(reserve_out)?;
        let denominator = reserve_in.checked_add(amount_in_less_fee)?;
        let amount_out = numerator.checked_div(denominator)?;
        // 5. Calculate price impact with decimal adjustment

        let current_price = reserve_out.checked_div(reserve_in)?;

        let new_reserve_in = reserve_in.checked_add(amount_in_less_fee)?;
        let new_reserve_out = reserve_out.checked_sub(amount_out)?;
        let new_price = new_reserve_out.checked_div(new_reserve_in)?;

        // Multiply numerator first to preserve precision (like fixed-point math)
        let scale = U256::from(10).pow(18.into()); // or 1e6 if 1e18 feels too big
        let current_price = reserve_out.checked_mul(scale)?.checked_div(reserve_in)?;
        let new_price = new_reserve_out
            .checked_mul(scale)?
            .checked_div(new_reserve_in)?;

        let price_impact = current_price
            .checked_sub(new_price)?
            .checked_mul(U256::from(10000))?
            .checked_div(current_price)?;

        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            fee: self.fee,
            token0: self.token0_addr,
            token1: self.token1_addr,
            pool: self.address,
            from0,
            amount_in,
            amount_out,
            price_impact,
            fee_amount: amount_in.checked_sub(amount_in_less_fee)?,
            raw_price: current_price,
        })
    }
}
