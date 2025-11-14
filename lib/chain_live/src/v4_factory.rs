use std::cell::RefCell;

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
    keccak256,
    map::{HashMap, HashSet},
};

use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use chain_db::{
    chains_db::ChainsDB,
    p_config::{AnyPoolConfig, V4Config},
    p_ticks::PoolWords,
    p_tokens::Tokens,
};
use futures::future::join_all;
use sol::sol_types::{PoolKey, StateView::StateViewInstance};

use crate::{
    any_pool::AnyPoolKey,
    v4_pool::{V4Data, V4Pool},
};

#[derive(Debug)]
pub struct V4Factory<P: Provider + Clone> {
    pub chain_id: u64,
    pub name: String,
    pub contract: StateViewInstance<P>,
    tried: RefCell<HashSet<AnyPoolConfig>>,
}

impl<P: Provider + Clone> V4Factory<P> {
    pub async fn search_pools(
        &self,
        token_a: Address,
        token_b: Address,
        found: HashSet<(V4Config, Tokens)>,
    ) -> Vec<V4Pool<P>> {
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

                    let pool_config = V4Config {
                        fee,
                        tick_spacing,
                        hooks: hook,
                        token0: a,
                        token1: b,
                    };

                    if found.contains(&(
                        pool_config,
                        Tokens {
                            a: Some(a),
                            b: Some(b),
                        },
                    )) {
                        continue;
                    }

                    let new = V4Data {
                        slot0: None,
                        liquidity: None,
                        ticks: PoolWords::default(),
                        pool_key: key.clone(),
                    };

                    let mut new_pool = V4Pool {
                        data: new,
                        //the contract is the same here
                        //this is not wrong
                        contract: self.contract.clone(),
                        id: keccak256(key.abi_encode()),
                    };

                    fut.push(async move { new_pool.sync_liquidity().await });
                    pools.push((pool_config, new_pool));
                }
            }
        }

        let results = join_all(fut).await;
        let mut real_pools = Vec::new();

        for (res, (config, pool)) in results.iter().zip(pools) {
            if let Ok(r) = res {
                real_pools.push(pool)
            } else {
                self.tried.borrow_mut().insert(AnyPoolConfig::V4(config));
            }
        }

        real_pools
    }
}
