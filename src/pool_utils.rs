use bigdecimal::BigDecimal;
use ethers::abi::ethabi;
use std::{fmt::Debug, sync::Arc};
use tokio::sync::RwLock;

use crate::pool::{Pool, V2Pool, V3Pool};


#[derive(Debug)]
pub enum AnyPool {
    V2(V2Pool),
    V3(V3Pool),
}

impl Pool for AnyPool {
    async fn update(&mut self) {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.update().await,
            AnyPool::V3(v3_pool) => v3_pool.update().await,
        }
    }

    fn trade(&self, amount_in: u32, from: bool) -> TradeData {
        match self {
            AnyPool::V2(v2_pool) => v2_pool.trade(amount_in, from),
            AnyPool::V3(v3_pool) => v3_pool.trade(amount_in, from),
        }
    }
}

#[derive(Debug)]
pub struct SomePools {
    pub pools: Vec<PoolDir>
}

impl SomePools {
    pub fn new(pools: Vec<PoolDir>) -> Self {
        SomePools { pools }
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

pub struct TradeData {
    pub from0: bool,
    pub amount_in: BigDecimal,
    pub amount_out: BigDecimal,
    pub price_impact: BigDecimal,
    pub fee: BigDecimal,
    pub raw_price: BigDecimal,
}

pub struct AbisData {
    pub v2_pool: ethabi::Contract,
    pub v3_pool: ethabi::Contract,
    pub v2_factory: ethabi::Contract,
    pub v3_factory: ethabi::Contract,
    pub bep_20: ethabi::Contract,
    
}