use std::collections::BTreeMap;

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
use futures::future::join_all;
use sol::sol_types::IUniswapV3Factory::IUniswapV3FactoryInstance;

use crate::v3_pool::V3Data;

#[derive(Debug)]
pub struct V3Factory<P: Provider + Clone> {
    pub name: String,
    pub contract: IUniswapV3FactoryInstance<P>,
}
// search for all possible pools for tokes
// order does not matter
impl<P: Provider + Clone> V3Factory<P> {
    pub async fn search_pools(&self, token_a: Address, token_b: Address) -> Vec<(Address, V3Data)> {
        let [a, b] = [token_a, token_b];

        let mut pools = Vec::new();

        let fees = vec![100, 250, 500, 1000, 1500, 2000, 2500, 3000, 5000, 10000];
        let tksp = vec![1, 5, 10, 20, 30, 40, 50, 60, 100, 200];
        let mut fut = Vec::new();

        for idx in 0..fees.len() {
            let fee = U24::from(fees[idx]);
            let ts = I24::try_from(tksp[idx]).unwrap();
            let _key = &V3Key { 0: (a, b, fee) };

            let mut p = V3Data {
                fee: Some(fee),
                tick_spacing: Some(ts),
                token0: Some(a),
                token1: Some(b),
                slot0: None,
                liquidity: None,
                ticks: BTreeMap::new(),
            };

            fut.push(async move { (self.contract.getPool(a, b, U24::from(fee)).call().await, p) });
        }

        let results = join_all(fut).await;

        for res in results {
            if let Ok(addr) = res.0 {
                if addr == Address::ZERO {
                    continue;
                }

                pools.push((addr, res.1));
            }
        }

        pools
    }
}
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct V3Key((Address, Address, U24));
