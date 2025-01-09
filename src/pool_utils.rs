use std::{fmt::Debug, sync::Arc};
use bigdecimal::BigDecimal;
use sqlx::pool;
use tokio::sync::RwLock;

use crate::pool::{Pool, V2Pool, V3Pool};

#[derive(Debug)]
pub struct SomePools {
    pub v2pools: Vec<PoolDir<V2Pool>>,
    pub v3pools: Vec<PoolDir<V3Pool>>,
}

impl SomePools {
    pub fn new(v2pools: Vec<PoolDir<V2Pool>>, v3pools: Vec<PoolDir<V3Pool>>) -> Self {
        SomePools { v2pools, v3pools }
    }
    pub fn add_v2pool(&mut self, pool: PoolDir<V2Pool>) {
        println!("added pool so some_pools");
        self.v2pools.push(pool);
    }

    pub fn add_v3pool(&mut self, pool: PoolDir<V3Pool>) {
        self.v3pools.push(pool);
    }

    pub fn get_all_v2pools(&self) -> &Vec<PoolDir<V2Pool>> {
        &self.v2pools
    }

    pub fn get_all_v3pools(&self) -> &Vec<PoolDir<V3Pool>> {
        &self.v3pools
    }
}

#[derive(Debug)]
pub struct PoolDir<T>
where
    T: Pool + Debug + Send + Sync + 'static, // Ensure `T` is thread-safe and has a valid lifetime.
{
    pub pool: Arc<RwLock<T>>,
    pub is0: bool,
}

impl<T> PoolDir<T>
where
    T: Pool + Debug + Send + Sync + 'static, // Ensure `T` is thread-safe and has a valid lifetime.
{
    pub fn new(pool: Arc<RwLock<T>>, is0: bool) -> Self{
        Self { pool: pool, is0: is0 }
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