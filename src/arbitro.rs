use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::{pool::{Pool, V2Pool}, pool_utils::{AnyPool, PoolDir, SomePools}};

#[derive(Debug)]
pub struct Arbitro {
    pub pools_by_token: Arc<RwLock<HashMap<String,Arc<RwLock<SomePools>>>>>,
}

impl Arbitro {
    pub fn new() -> Self {
        Self { pools_by_token: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn add_v2(&mut self, token: String, pool: Arc<RwLock<V2Pool>> ,is0: bool) {
        let mut map = self.pools_by_token.write().await;

        let any_pool = AnyPool::V2(pool.clone());

        let pool_dir = PoolDir::new(any_pool,is0);
       
        let some_pools = map
        .entry(token.to_string())
        .or_insert_with(| | Arc::new(RwLock::new(SomePools::new(vec![]))));

        some_pools.write().await.add_pool(pool_dir);
    }   

    pub async fn pathfind(& self, start_address: &str,start_in: u128){
        let _some_pools = {
            // Lock only to get the SomePools instance
            let map: tokio::sync::RwLockReadGuard<'_, HashMap<String, Arc<RwLock<SomePools>>>> = self.pools_by_token.read().await;
            let pools: Arc<RwLock<SomePools>> = map.get(start_address).cloned().unwrap(); // Clone or reference the data
            pools
        };

        let some_pools = _some_pools.read().await;

    }
    pub async fn print(&self){
        let pools = self.pools_by_token.read().await;
        println!("{}" ,pools.len());
        // Iterate through each token and display the count of pools for each
        for (token, pools_arc) in pools.iter() {
            let pool_count = pools_arc.read().await.pools.len();
            // Here you can get the length of the pools or any other detail
            println!("{} has {} pools", token, pool_count);
        }
        
    }
}