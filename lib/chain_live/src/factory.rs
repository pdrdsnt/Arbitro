use alloy_primitives::{Address, map::HashSet};
use alloy_provider::Provider;
use chain_db::{
    chains_db::ChainsDB, p_config::AnyPoolConfig, p_key::SledPairKey, p_tokens::Tokens,
};
use tokio::select;

use crate::factory_context::SearchContext;

pub const FEES: [u32; 10] = [100, 250, 500, 1000, 1500, 2000, 2500, 3000, 5000, 10000];
pub const TICK_SPACES: [u32; 10] = [1, 5, 10, 20, 30, 40, 50, 60, 100, 200];

pub trait Factory<P: Provider + Clone> {
    fn get_space(&self) -> Vec<AnyPoolConfig>;
    fn get_ctx(&self) -> &SearchContext;
    fn get_targets(&self) -> Vec<SledPairKey>;
    fn create_pairs(&self) -> Vec<SledPairKey> {}

    fn create_calls(&self) {
        let arr = self.get_space();

        for any_config in arr {
            match any_config {
                AnyPoolConfig::V2(v2_config) => {}
                AnyPoolConfig::V3(v3_config) => {}
                AnyPoolConfig::V4(v4_config) => {}
            }
        }
    }
}
