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

use num_traits::{real::Real, FromPrimitive, ToPrimitive};
use sqlx::pool;
use tokio::sync::RwLock;

use crate::mult_provider::MultiProvider;

#[derive(Debug, Clone)]
pub struct Token {
    pub name: String,
    pub address: H160,
    pub symbol: String,
    pub decimals: u8,
    pub contract: Contract<Provider<MultiProvider>>,
}

impl Token {
    pub fn new(
        name: String,
        address: H160,
        symbol: String,
        decimals: u8,
        contract: Contract<Provider<MultiProvider>>,
       
    ) -> Self {
        Token {
            name,
            address,
            symbol,
            contract,
            decimals,
        }
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
