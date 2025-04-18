use core::f32;
use std::{
    collections::{HashMap, HashSet},
    i32,
    sync::Arc,
    time::{Duration, Instant},
};

use bigdecimal::BigDecimal;
use ethers::{
    contract::Contract,
    core::k256::sha2::digest::Output,
    providers::{Http, Provider, Ws},
    types::{H160, U256},
};
use graph::graph::IntoConnections;
use num_traits::{real::Real, FromPrimitive, ToPrimitive};
use sqlx::pool;
use tokio::sync::RwLock;

use crate::{
    arbitro::{self, Arbitro},
    pool::{Pool, V2Pool},
    pool_utils::{AnyPool, PoolDir, Trade},
};

#[derive(Debug, Clone)]
pub struct Token {
    pub name: String,
    pub address: H160,
    pub symbol: String,
    pub decimals: u8,
    pub contract: Contract<Provider<Http>>,
    pub pools: HashMap<H160, PoolDir>,
    pub pools_by_pair: HashMap<H160, HashSet<H160>>,
}

impl Token {
    pub fn new(
        name: String,
        address: H160,
        symbol: String,
        decimals: u8,
        contract: Contract<Provider<Http>>,
        pools: HashMap<H160, PoolDir>,
    ) -> Self {
        Token {
            name,
            address,
            symbol,
            contract,
            decimals,
            pools,
            pools_by_pair: HashMap::new(),
        }
    }

    pub async fn add_pool(&mut self, pool: Arc<RwLock<AnyPool>>, is0: bool) {
        let pool_read = pool.read().await;
        let pool_tokens = pool_read.get_tokens();
        let pool_dir = PoolDir::new(pool.clone(), is0);
        let other_node = if !is0 { pool_tokens[0] } else { pool_tokens[1] };

        let pool_address = match &*pool_read {
            AnyPool::V2(v2_pool) => v2_pool.address.clone(),
            AnyPool::V3(v3_pool) => v3_pool.address.clone(),
        };

        let pool_version = match &*pool_read {
            AnyPool::V2(v2_pool) => v2_pool.version.clone(),
            AnyPool::V3(v3_pool) => v3_pool.version.clone(),
        };

        let pool_name = match &*pool_read {
            AnyPool::V2(v2_pool) => v2_pool.exchange.clone(),
            AnyPool::V3(v3_pool) => v3_pool.exchange.clone(),
        };

        let pool_fee = match &*pool_read {
            AnyPool::V2(v2_pool) => v2_pool.fee.clone(),
            AnyPool::V3(v3_pool) => v3_pool.fee.clone(),
        };

        match self.pools_by_pair.get_mut(&other_node) {
            Some(mut pools) => {
                pools.insert(pool_address.clone());
            }
            None => {
                let mut new_pools = HashSet::new();
                new_pools.insert(pool_address.clone());
                self.pools_by_pair.insert(other_node, new_pools);
            }
        };

        println!(
            "added pool {} {} {} {} to token {} {} from0: {}",
            &pool_address, pool_name, pool_version, pool_fee, self.name, self.address, is0
        );

        self.pools
            .entry(pool_address)
            .and_modify(|existing| {
                *existing = pool_dir.clone();
            })
            .or_insert(pool_dir);
    }

    pub async fn update_pools(&mut self) {}

    pub fn trades(&self, input: U256) -> HashMap<H160, Vec<Trade>> {
        let mut output = HashMap::new();
        let timeout = Duration::from_secs(2); // Adjust as needed

        for (_, pool_dir) in &self.pools {
            let pool_arc: Arc<RwLock<AnyPool>> = Arc::clone(&pool_dir.pool);
            let start = Instant::now();
            let mut pool_read = None;

            // Attempt to acquire read lock with timeout
            while Instant::now().duration_since(start) < timeout {
                if let Ok(guard) = pool_arc.try_read() {
                    pool_read = Some(guard);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10)); // Backoff
            }

            let pool_read = match pool_read {
                Some(guard) => guard,
                None => {
                    println!("Timeout acquiring lock for pool");
                    continue;
                }
            };
            // Debug logging (consider using log crate for production)
            println!(
                "trying trade in: {:?} {:?} {:?}",
                pool_read.get_dex(),
                pool_read.get_version(),
                pool_read.get_fee()
            );

            if let Some(trade_data) = pool_read.trade(input, pool_dir.is0) {
                // Determine output token address
                let output_token = if trade_data.from0 {
                    trade_data.token1.clone()
                } else {
                    trade_data.token0.clone()
                };

                // Entry API with reference to avoid cloning
                output
                    .entry(output_token)
                    .or_insert_with(Vec::new)
                    .push(trade_data);
            }
        }

        // Sort trades by descending amount_out using U256's native comparison
        for trades in output.values_mut() {
            trades.sort_by(|a, b| b.amount_out.cmp(&a.amount_out));
        }

        output
    }

    pub fn best_trade(&self, input: U256) -> Option<Trade> {
        let mut trade = None;
        let mut bigger_out = U256::from(0);
        for (addr, pool) in self.pools.iter() {
            let pool_read = pool.pool.blocking_read();
            match pool_read.trade(input, pool.is0) {
                Some(_trade) => {
                    if _trade.amount_out > bigger_out {
                        bigger_out = _trade.amount_out.clone();
                        trade = Some(_trade);
                    }
                }
                None => (),
            };
        }
        trade
    }
}
/*
impl IntoConnections<H160, float32> for Token {
    type Item = Trade;
    fn into_connections(
        &self,
        map: &HashMap<H160, Arc<std::sync::RwLock<Self>>>,
        input: float32,
    ) -> Vec<Self::Item> {
        println!("into_connections called on token: {}", self.name);
        let _ = map;
        let mut trades = Vec::<Trade>::new();
        for (_, pool_dir) in &self.pools {
            let pool_read = pool_dir.pool.try_read().unwrap();
            //if let Some(trade_data) = pool_read.trade(input, pool_dir.is0)
            //{
            //    trades.push(trade_data);
            //}
        }
        for t in &trades {
            let _t = if !t.from0 { t.token0 } else { t.token1 };
            if let Some(target_node) = map.get(&_t) {
                if let Ok(mut w) = target_node.try_write() {
                } else {
                    println!("failed to acquire write lock");
                }
            } else {
                println!("target node not found");
            }
            println!("connection to: {}", _t);
            println!(
                "cost: {:?}",
                t.amount_out.to_string().parse::<f32>().unwrap()
            );
            println!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        }
        println!("===================================================");
        trades
    }

    fn update(&self, h: float32) {
        unimplemented!()
    }
}
 */
impl Eq for Token {}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
