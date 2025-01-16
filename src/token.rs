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
    pub pools: SomePools,
}

impl Token {
    pub fn new(
        name: String,
        address: H160,
        symbol: String,
        contract: Contract<Provider<Ws>>,
        pools: SomePools,
    ) -> Self {
        Token {
            name,
            address,
            symbol,
            contract,
            pools,
        }
    }

    pub fn add_pool(&mut self, pool: Arc<RwLock<AnyPool>>, is0: bool) {
        
        let pool_dir = PoolDir::new(pool.clone(), is0);

        self.pools.add_pool(pool_dir);
    }

    pub async fn update_pools(&mut self){

    }


}
