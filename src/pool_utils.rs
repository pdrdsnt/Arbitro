use bigdecimal::BigDecimal;
use ethers::{abi::ethabi, types::H160};
use std::{fmt::Debug, i128, sync::Arc};
use tokio::sync::RwLock;

use crate::{pathfinder::pathfinder::{Edge, Heuristic}, pool::{Pool, V2Pool, V3Pool}};


#[derive(Debug)]
pub enum AnyPool {
    V2(V2Pool),
    V3(V3Pool),
}

impl AnyPool {
    pub fn is_0(&self,addr: H160) -> bool {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.token0 == addr,
            AnyPool::V3(v3_pool) => v3_pool.token0 == addr,
        }
    }
}

impl Pool for AnyPool {
    async fn update(&mut self) {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.update().await,
            AnyPool::V3(v3_pool) => v3_pool.update().await,
        }
    }

    fn trade(&self, amount_in: u32, from: bool) -> Trade{
        match self {
            AnyPool::V2(v2_pool) => v2_pool.trade(amount_in, from),
            AnyPool::V3(v3_pool) => v3_pool.trade(amount_in, from),
        }
    }
}

#[derive(Debug)]
pub struct TokenPools {
    pub pools: Vec<PoolDir>
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

#[derive(Debug)]
pub struct PoolDir
{
    pub pool: Arc<RwLock<AnyPool>>,
    pub is0: bool,
}

impl PoolDir
{
    pub fn new(pool: Arc<RwLock<AnyPool>>, is0: bool) -> Self {
        Self {
            pool,
            is0,
        }
    }
}

pub struct AbisData {
    pub v2_pool: ethabi::Contract,
    pub v3_pool: ethabi::Contract,
    pub v2_factory: ethabi::Contract,
    pub v3_factory: ethabi::Contract,
    pub bep_20: ethabi::Contract,
    
}

#[derive(Clone)]
pub struct Trade {
    pub token0: H160,
    pub token1: H160,
    pub pool: H160,
    pub from0: bool,
    pub amount_in: BigDecimal,
    pub amount_out: BigDecimal,
    pub price_impact: BigDecimal,
    pub fee: BigDecimal,
    pub raw_price: BigDecimal,
}

impl Heuristic<i128> for Trade {
    fn get_h(self) -> i128 {
        let big_decimal = (&self.amount_in - &self.amount_out) * BigDecimal::from(1000);
        let scaled_amount = big_decimal;

        let bg = scaled_amount.as_bigint_and_exponent().0;
        let bites = bg.to_signed_bytes_le();
        let mut i128_bytes: [u8; 16] = [0; 16]; 
        let bytes = bg.to_signed_bytes_le(); 
    
        for (i, &byte) in bytes.iter().enumerate() {
            i128_bytes[15 - i] = byte;
        }
    
        i128::from_be_bytes(i128_bytes)      
    }
}

impl Into<Edge<H160,i128>> for Trade {  
    fn into(self) -> Edge<H160,i128> {
        Edge{
            a: if self.from0 {self.token0} else {self.token1},
            b: if self.from0 {self.token0} else {self.token1},
            i: self.pool,
            h: 2,
        }
    }
}