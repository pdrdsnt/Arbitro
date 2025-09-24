use alloy_primitives::{
    Address, B256,
    aliases::{I24, U24},
    keccak256,
};

use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use futures::future::join_all;
use sol::sol_types::{PoolKey, StateView::StateViewInstance};
use std::collections::BTreeMap;

use crate::v4_pool::{V4Data, V4Pool};

#[derive(Debug)]
pub struct V4Factory<P: Provider + Clone> {
    pub name: String,
    pub contract: StateViewInstance<P>,
}

impl<P: Provider + Clone> V4Factory<P> {
    pub async fn search_pools(&self, token_a: Address, token_b: Address) -> Vec<V4Pool<P>> {
        let [a, b] = [token_a, token_b];

        let mut pools = Vec::new();

        let fees = vec![100, 250, 500, 1000, 1500, 2000, 2500, 3000, 5000, 10000];
        let hooks = vec![Address::ZERO];
        let spacings = vec![1, 5, 10, 20, 30, 40, 50, 60, 100, 200, 400, 1000];
        let mut fut = Vec::new();

        for ifee in 0..fees.len() {
            for itickspacing in 0..spacings.len() {
                for ihook in 0..hooks.len() {
                    let hook = hooks[ihook];
                    let fee = U24::from(fees[ifee]);
                    let tick_spacing = I24::try_from(spacings[itickspacing]).unwrap();

                    let key = PoolKey {
                        currency0: a,
                        currency1: b,
                        fee,
                        tickSpacing: tick_spacing,
                        hooks: hook,
                    };

                    let new = V4Data {
                        slot0: None,
                        liquidity: None,
                        ticks: BTreeMap::new(),
                        pool_key: key.clone(),
                    };
                    let new_pool = V4Pool {
                        data: new,
                        contract: self.contract.clone(),
                        id: keccak256(key.abi_encode()),
                    };

                    fut.push(async move { new_pool.try_fill_pool_rpc().await });
                }
            }
        }

        let results = join_all(fut).await;

        for res in results {
            let pool = res;
            if pool.data.slot0.is_some() {
                pools.push(pool)
            }
        }

        pools
    }
}
