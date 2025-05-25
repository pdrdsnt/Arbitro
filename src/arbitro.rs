use std::collections::HashMap;

use ethers::types::H160;

use crate::{chain_src::ChainSrc, mapped_vec::MappedVec, token::Token, v3_pool_sim::V3PoolSim, v_pool_sim::AnyPoolSim};

pub struct Arbitro {
    pub pools: MappedVec<AnyPoolSim>,
    pub pools_by_token: HashMap<H160, Vec<(H160, bool)>>,
    pub tokens_by_pool: HashMap<H160, Vec<(H160, H160)>>,
}

impl Arbitro {
    pub fn new(c_src: MappedVec<AnyPoolSim>) -> Self {
        let mut pools = MappedVec::new();
        let mut pools_by_token = HashMap::new();
        let mut tokens_by_pool = HashMap::new();

        for (pool_addr, pool) in c_src.iter() {

            let tokens = pool.get_tokens();
            

            for token in tokens {
                pools_by_token
                    .entry(token)
                    .or_insert_with(Vec::new)
                    .push((*pool_addr, pool.is_0(&token)));
            }

            tokens_by_pool.insert(*pool_addr, vec![(tokens[0], tokens[1])]);
        }

        Self {
            pools,
            pools_by_token,
            tokens_by_pool,
        }
    }
}
