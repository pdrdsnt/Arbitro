use bigdecimal::BigDecimal;
use ethers::{
    abi::ethabi,
    types::{H160, U256},
};
use graph::{
    edge::Edge,
    graph::{Connection, IntoConnections},
};
use num_traits::{FromPrimitive, Zero};
use std::{fmt::Debug, i128, ops::{Div, Mul}, str::FromStr, string, sync::Arc};
use tokio::sync::RwLock;

use crate::pool::{Pool, PoolUpdateError};
use crate::v2_pool::V2Pool;
use crate::v3_pool::V3Pool;

#[derive(Debug)]
pub enum AnyPool {
    V2(V2Pool),
    V3(V3Pool),
}

impl AnyPool {
    pub fn get_other(&self, token: &H160) -> Option<H160> {
        match self {
            AnyPool::V2(v2_pool) => {
                if v2_pool.token0.address == *token {
                    Some(v2_pool.token1.address.clone())
                } else if v2_pool.token1.address == *token {
                    Some(v2_pool.token0.address.clone())
                } else {
                    None
                }
            }
            AnyPool::V3(v3_pool) => {
                if v3_pool.token0.address == *token {
                    Some(v3_pool.token1.address.clone())
                } else if v3_pool.token1.address == *token {
                    Some(v3_pool.token0.address.clone())
                } else {
                    None
                }
            }
        }
    }
    pub fn get_tokens(&self) -> [H160; 2] {
        match self {
            AnyPool::V2(v2_pool) => [v2_pool.token0.address, v2_pool.token1.address],
            AnyPool::V3(v3_pool) => [v3_pool.token0.address, v3_pool.token1.address],
        }
    }
    pub fn is_0(&self, addr: &H160) -> bool {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.token0.address == *addr,
            AnyPool::V3(v3_pool) => v3_pool.token0.address == *addr,
        }
    }
    pub fn in_pool(&self, addr: H160) -> bool {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.token0.address == addr,
            AnyPool::V3(v3_pool) => v3_pool.token0.address == addr,
        }
    }
    pub fn get_address(&self) -> H160 {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.address,
            AnyPool::V3(v3_pool) => v3_pool.address,
        }
    }
    pub fn get_fee(&self) -> u32 {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.fee,
            AnyPool::V3(v3_pool) => v3_pool.fee,
        }
    }
    pub fn get_version(&self) -> String {
        match self {
            AnyPool::V2(_) => "v2".to_string(),
            AnyPool::V3(_) => "v3".to_string(),
        }
    }
    pub fn get_dex(&self) -> String {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.exchange.clone(),
            AnyPool::V3(v3_pool) => v3_pool.exchange.clone(),
        }
    }
    
    pub fn get_price(&self, decimals: [u8; 2]) -> Option<f64> {
        match self {
            AnyPool::V2(v2_pool) => {
                println!("V2 pool price calculation");
                println!("Reserves: {:?}", v2_pool.reserves0);
                println!("Reserves: {:?}", v2_pool.reserves1);
                let raw_price = Self::get_v2_raw_price(&v2_pool.reserves0, &v2_pool.reserves1)?;
                Self::adjust_for_decimals(raw_price, decimals)
            }
            AnyPool::V3(v3_pool) => {
                let raw_price = Self::get_v3_raw_price(v3_pool.x96price)?;
                Self::adjust_for_decimals(raw_price, decimals)
            }
        }
    }
    
    // V2-specific price calculation
    fn get_v2_raw_price(reserves0: &U256, reserves1: &U256) -> Option<BigDecimal> {
        let r0 = Self::big_decimal_from_u256(reserves0)?;
        let r1 = Self::big_decimal_from_u256(reserves1)?;
        
        if r0.is_zero() {
            return None;
        }
        
        Some(r1 / r0)
    }
    
    // V3-specific price calculation
    fn get_v3_raw_price(price_x96: U256) -> Option<BigDecimal> {
        if price_x96.is_zero() {
            return None;
        }
        
        let numerator = Self::big_decimal_from_u256(&price_x96)?;
        let denominator = Self::big_decimal_from_u128(1u128 << 96)?;
        
        Some(numerator / denominator)
    }
    
    // Decimal adjustment logic
    pub fn adjust_for_decimals(raw_price: BigDecimal, decimals: [u8; 2]) -> Option<f64> {
        let exp = (decimals[1] as i32) - (decimals[0] as i32);
        let abs_exp = exp.abs() as u32;
        let factor = 10u64.pow(abs_exp);
        
        let adjusted_price = if exp >= 0 {
            raw_price * BigDecimal::from(factor)
        } else {
            raw_price / BigDecimal::from(factor)
        };
        
        adjusted_price.to_f64().or(Some(0.0))
    }
    
    // Helper function for safe U256 to BigDecimal conversion
    fn big_decimal_from_u256(value: &U256) -> Option<BigDecimal> {
        BigDecimal::from_str(&value.to_string()).ok()
    }
    
    // Helper function for safe u128 to BigDecimal conversion
    fn big_decimal_from_u128(value: u128) -> Option<BigDecimal> {
        BigDecimal::from_str(&value.to_string()).ok()
    }
    
    
    
    pub fn get_reserves(&self) -> Option<(U256, U256)> {
        match self {
            AnyPool::V2(v2_pool) => {
                // Assuming V2Pool has an async get_reserves() -> (U256, U256)
                Some((v2_pool.reserves0, v2_pool.reserves1))
            }
            AnyPool::V3(v3_pool) => {
                // Assuming V3Pool has an async get_reserves() -> (U256, U256)
                let Q96 = 2u128.pow(96);
                let liquidity = v3_pool.liquidity;
                let sqrt_price_x96 = v3_pool.x96price;
                // 2) reserve0 = liquidity * 2^96 / sqrtPriceX96
                let numerator0 = liquidity.saturating_mul(U256::one() << 96);
                let r0 = numerator0
                    .checked_div(sqrt_price_x96)
                    .unwrap_or(U256::zero());

                // 3) reserve1 = liquidity * sqrtPriceX96 / 2^96
                let r1 = liquidity.saturating_mul(sqrt_price_x96) >> 96;

                Some((r0, r1))
            }
        }
    }

}
impl Pool for AnyPool {
    async fn update(&mut self) -> Result<(), PoolUpdateError> {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.update().await,
            AnyPool::V3(v3_pool) => v3_pool.update().await,
        }
    }

    fn trade(&self, amount_in: U256, from: bool) -> Option<Trade> {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.trade(amount_in, from),
            AnyPool::V3(v3_pool) => v3_pool.trade(amount_in, from),
        }
    }
}

#[derive(Debug)]
pub struct TokenPools {
    pub pools: Vec<PoolDir>,
}

impl TokenPools {
    pub fn new(pools: Vec<PoolDir>) -> Self {
        TokenPools { pools }
    }
    pub fn add_pool(&mut self, pool: PoolDir) {
        println!("added pool so some_pools");
        self.pools.push(pool);
    }
}

#[derive(Debug, Clone)]
pub struct PoolDir {
    pub pool: Arc<RwLock<AnyPool>>,
    pub is0: bool,
}

impl PoolDir {
    pub fn new(pool: Arc<RwLock<AnyPool>>, is0: bool) -> Self {
        Self { pool, is0 }
    }
}

pub struct AbisData {
    pub v2_pool: ethabi::Contract,
    pub v3_pool: ethabi::Contract,
    pub v2_factory: ethabi::Contract,
    pub v3_factory: ethabi::Contract,
    pub bep_20: ethabi::Contract,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Trade {
    pub dex: String,
    pub version: String,
    pub fee: u32,
    pub token0: H160,
    pub token1: H160,
    pub pool: H160,
    pub from0: bool,
    pub amount_in: ethers::types::U256,
    pub amount_out: ethers::types::U256,
    pub price_impact: U256,
    pub fee_amount: U256,
    pub raw_price: U256,
}

use bigdecimal::ToPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick {
    pub tick: i32,
    pub liquidityNet: i128,
}
