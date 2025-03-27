use bigdecimal::BigDecimal;
use ethers::{abi::ethabi, types::{H160, U256}};
use graph::{
    edge::Edge,
    graph::{Connection, IntoConnections},
};
use std::{fmt::Debug, i128, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    pool::{Pool, V2Pool, V3Pool},
};

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
            },
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
            AnyPool::V2(v2_pool) => {
                v2_pool.token0.address == *addr
            }
            AnyPool::V3(v3_pool) => {
                v3_pool.token0.address == *addr
            }
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
}

impl Pool for AnyPool {
    async fn update(&mut self) {
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

#[derive(Clone, Debug, PartialEq, PartialOrd,Eq, Ord)]
pub struct Trade {
    pub token0: H160,
    pub token1: H160,
    pub pool: H160,
    pub from0: bool,
    pub amount_in: ethers::types::U256,
    pub amount_out: ethers::types::U256,
    pub price_impact: U256,
    pub fee: U256,
    pub raw_price: U256,
}

use bigdecimal::ToPrimitive;


#[derive(Debug)]
pub struct Tick {
    pub tick: i32,
    pub liquidityNet: i128,
}
