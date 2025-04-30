use ethers::{abi::Address, contract::Contract, types::U256};
use ethers_providers::Provider;

use crate::{mult_provider::MultiProvider, pool::{Pool, PoolUpdateError}, pool_utils::Trade, token::Token};


#[derive(Debug)]
pub struct V2Pool {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub reserves0: U256,
    pub reserves1: U256,
    pub contract: Contract<Provider<MultiProvider>>,
}

impl V2Pool {
    // Private constructor
    async fn new(
        exchange: String,
        version: String,
        fee: u32,
        address: Address,
        token0: Token,
        token1: Token,
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
        token0: Token,
        token1: Token,
        contract: Contract<Provider<MultiProvider>>,
    ) -> Self {
        let mut instance =
            V2Pool::new(exchange, version, fee, address, token0, token1, contract).await;
        instance.update().await;
        instance
    }
}


impl Pool for V2Pool {
    async fn update(&mut self) -> Result<(), PoolUpdateError> {
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
                    Err(erro) => {println!("contract call error {}", erro);
                        return Err(PoolUpdateError::from(erro));
                    }
                }},
            Err(erro) => {
                println!("abi erro {}", erro);
                return Err(PoolUpdateError::from(erro));
            },
        }
    }

    fn trade(&self, amount_in: U256, from0: bool) -> Option<Trade> {
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
            token0: self.token0.address,
            token1: self.token1.address,
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