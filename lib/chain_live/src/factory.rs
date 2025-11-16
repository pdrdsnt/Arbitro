use alloy_primitives::map::HashSet;
use alloy_provider::Provider;
use chain_db::{p_config::AnyPoolConfig, p_tokens::Tokens};
use sol::sol_types::{IUniswapV2Pair::IUniswapV2PairInstance, V3Pool::V3PoolInstance};

use crate::{
    any_pool::AnyPool, v2_factory::V2Factory, v2_pool::V2Pool, v3_factory::V3Factory,
    v3_pool::V3Pool, v4_factory::V4Factory, v4_pool::V4Pool,
};

pub trait Factory<P: Provider + Clone> {
    async fn flatout(&self, pairs: &Vec<Tokens>, out: HashSet<AnyPoolConfig>)
    -> Vec<AnyPoolConfig>;
}
