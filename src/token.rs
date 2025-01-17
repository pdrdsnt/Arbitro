use std::{collections::HashMap, sync::Arc};

use ethers::{
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
};
use tokio::sync::RwLock;

use crate::{
    pathfinder::pathfinder::Edge,
    pool::{Pool, V2Pool},
    pool_utils::{AnyPool, PoolDir, Trade},
};

pub struct Token {
    pub name: String,
    pub address: H160,
    pub symbol: String,
    pub contract: Contract<Provider<Ws>>,
    pub pools: HashMap<H160, PoolDir>,
}

impl Token {
    pub fn new(
        name: String,
        address: H160,
        symbol: String,
        contract: Contract<Provider<Ws>>,
        pools: HashMap<H160, PoolDir>,
    ) -> Self {
        Token {
            name,
            address,
            symbol,
            contract,
            pools,
        }
    }

    pub async fn add_pool(&mut self, pool: Arc<RwLock<AnyPool>>, is0: bool) {
        let pool_dir = PoolDir::new(pool.clone(), is0);

        let key = match &*pool.read().await {
            AnyPool::V2(v2_pool) => v2_pool.address.clone(),
            AnyPool::V3(v3_pool) => v3_pool.address.clone(),
        };

        self.pools.entry(key).or_insert(pool_dir);
    }

    pub async fn update_pools(&mut self) {}
}

impl IntoIterator for Token {
    type Item = Trade;

    type IntoIter = std::vec::IntoIter<Trade>;

    fn into_iter(self) -> Self::IntoIter {

        let mut edges: Vec<Trade> = vec![];

        let _ = self.pools.iter()
        .map(|addr_and_pool| {
            let pool_read = &*addr_and_pool.1.pool.try_read().unwrap();
            let trade_data = pool_read.trade(100, pool_read.is_0(self.address));
            edges.push(trade_data);
        });

        edges.into_iter()
        
    }
}
