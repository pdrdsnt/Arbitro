use std::sync::Arc;

use ethers::{
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
};
use tokio::sync::RwLock;

use crate::{
    pool::V2Pool,
    pool_utils::{AnyPool, PoolDir, SomePools},
};

pub struct Token {
    pub name: String,
    pub address: H160,
    pub symbol: String,
    pub contract: Contract<Provider<Ws>>,
    pub pools: Arc<RwLock<SomePools>>,
}

impl Token {
    pub fn new(
        name: String,
        address: H160,
        symbol: String,
        contract: Contract<Provider<Ws>>,
        pools: Arc<RwLock<SomePools>>,
    ) -> Self {
        Token {
            name,
            address,
            symbol,
            contract,
            pools,
        }
    }

    pub async fn add_v2(&mut self, token: String, pool: Arc<RwLock<V2Pool>>, is0: bool) {
        
        let any_pool = AnyPool::V2(pool.clone());

        let pool_dir = PoolDir::new(any_pool, is0);

        self.pools.write().await.add_pool(pool_dir);
    }

    pub async fn update_pools(&mut self){

    }


}
