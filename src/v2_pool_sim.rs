use ethers::{abi::Address, contract::Contract, types::U256};
use ethers_providers::Provider;

use crate::{mult_provider::MultiProvider, token::Token, trade::Trade};

#[derive(Debug,)]
pub struct V2PoolSim {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub reserves0: U256,
    pub reserves1: U256,
}

impl V2PoolSim {
    // Private constructor
    pub fn new(
        exchange: String, version: String, fee: u32, address: Address, token0: Token, token1: Token, reserves0: U256,
        reserves1: U256,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange,
            version,
            fee,
            reserves0,
            reserves1,
        }
    }

    pub fn trade(&mut self, amount_in: U256, from0: bool,) -> Option<Trade,> {
        if (from0 && self.reserves0 == U256::zero()) || (!from0 && self.reserves1 == U256::zero()) {
            return None;
        }

        // 2. Get reserves in proper decimal scale
        let (mut reserve_in, mut reserve_out,) = match from0 {
            true => (self.reserves0, self.reserves1,),
            false => (self.reserves1, self.reserves0,),
        };

        // 3. Apply V2 fee calculation correctly (0.3% fee)
        let amount_in_less_fee = amount_in.checked_mul(U256::from(997,),)?.checked_div(U256::from(1000,),)?;
        let numerator = amount_in_less_fee.checked_mul(reserve_out,)?;
        let denominator = reserve_in.checked_add(amount_in_less_fee,)?;
        let amount_out = numerator.checked_div(denominator,)?;
        // 5. Calculate price impact with decimal adjustment

        let current_price = reserve_out.checked_div(reserve_in,)?;

        let new_reserve_in = reserve_in.checked_add(amount_in_less_fee,)?;
        let new_reserve_out = reserve_out.checked_sub(amount_out,)?;
        let new_price = new_reserve_out.checked_div(new_reserve_in,)?;

        // Multiply numerator first to preserve precision (like fixed-point math)
        let scale = U256::from(10,).pow(18.into(),); // or 1e6 if 1e18 feels too big
        let current_price = reserve_out.checked_mul(scale,)?.checked_div(reserve_in,)?;
        let new_price = new_reserve_out.checked_mul(scale,)?.checked_div(new_reserve_in,)?;

        let price_impact = current_price
            .checked_sub(new_price,)?
            .checked_mul(U256::from(10000,),)?
            .checked_div(current_price,)?;

        // Commit state
        if from0 {
            self.reserves0 = new_reserve_in;
            self.reserves1 = new_reserve_out;
        } else {
            self.reserves1 = new_reserve_in;
            self.reserves0 = new_reserve_out;
        }

        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            fee: self.fee,
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in,
            amount_out,
            price_impact,
            fee_amount: amount_in.checked_sub(amount_in_less_fee,)?,
            raw_price: current_price,
        },)
    }

    /// Apply an on-chain Swap event: update reserves exactly by logged amounts
    pub fn apply_swap(&mut self, amount0_in: U256, amount1_in: U256, amount0_out: U256, amount1_out: U256,) {
        self.reserves0 = self
            .reserves0
            .checked_add(amount0_in,)
            .unwrap_or(self.reserves0,)
            .checked_sub(amount0_out,)
            .unwrap_or(U256::zero(),);
        self.reserves1 = self
            .reserves1
            .checked_add(amount1_in,)
            .unwrap_or(self.reserves1,)
            .checked_sub(amount1_out,)
            .unwrap_or(U256::zero(),);
    }

    /// Mint (add liquidity) to the pool: both reserves increase
    pub fn mint(&mut self, amount0: U256, amount1: U256,) {
        self.reserves0 = self.reserves0.checked_add(amount0,).unwrap_or(self.reserves0,);
        self.reserves1 = self.reserves1.checked_add(amount1,).unwrap_or(self.reserves1,);
    }

    /// Burn (remove liquidity) from the pool: both reserves decrease
    pub fn burn(&mut self, amount0: U256, amount1: U256,) {
        self.reserves0 = self.reserves0.checked_sub(amount0,).unwrap_or(U256::zero(),);
        self.reserves1 = self.reserves1.checked_sub(amount1,).unwrap_or(U256::zero(),);
    }
}
