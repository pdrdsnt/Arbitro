use alloy_provider::Provider;
use chain_db::sled_pool_parts::PoolTokens;
use sol::sol_types::{IUniswapV2Pair::IUniswapV2PairInstance, V3Pool::V3PoolInstance};

use crate::{
    any_pool::AnyPool, v2_factory::V2Factory, v2_pool::V2Pool, v3_factory::V3Factory,
    v3_pool::V3Pool, v4_factory::V4Factory, v4_pool::V4Pool,
};

pub trait Factory<P: Provider + Clone> {
    async fn search_pairs(&self, pairs: &Vec<PoolTokens>) -> Vec<AnyPool<P>>;
}

impl<P: Provider + Clone> Factory<P> for V2Factory<P> {
    async fn search_pairs(&self, pairs: &Vec<PoolTokens>) -> Vec<AnyPool<P>> {
        let mut pools = vec![];
        for pair in pairs {
            if let (Some(a), Some(b)) = (pair.a, pair.b) {
                if let Some(v2) = V2Factory::search_pool(&self, a, b).await {
                    let v2_contract =
                        IUniswapV2PairInstance::new(v2.0, self.contract.provider().clone());

                    let pool = AnyPool::V2(V2Pool {
                        contract: v2_contract,
                        data: v2.1,
                    });

                    pools.push(pool);
                };
            }
        }

        pools
    }
}

impl<P: Provider + Clone> Factory<P> for V3Factory<P> {
    async fn search_pairs(&self, pairs: &Vec<PoolTokens>) -> Vec<AnyPool<P>> {
        let mut any_pools: Vec<AnyPool<P>> = Vec::new();

        for pair in pairs {
            if let (Some(a), Some(b)) = (pair.a, pair.b) {
                let pools = V3Factory::search_pools(&self, a, b).await;

                for p in pools {
                    let contract = V3PoolInstance::new(p.0, self.contract.provider().clone());
                    let pool = V3Pool::new(contract, p.1);

                    let any_pool = AnyPool::V3(pool);
                    any_pools.push(any_pool);
                }
            }
        }
        any_pools
    }
}

impl<P: Provider + Clone> Factory<P> for V4Factory<P> {
    async fn search_pairs(&self, pairs: &Vec<PoolTokens>) -> Vec<AnyPool<P>> {
        let mut any_pools: Vec<AnyPool<P>> = Vec::new();
        for pair in pairs {
            if let (Some(a), Some(b)) = (pair.a, pair.b) {
                let pools: Vec<V4Pool<P>> = V4Factory::search_pools(&self, a, b).await;

                for p in pools {
                    any_pools.push(AnyPool::V4(p));
                }
            }
        }
        any_pools
    }
}
