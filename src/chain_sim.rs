use std::collections::HashMap;

use ethers::types::H160;

use crate::{chain_src::ChainSrc, mapped_vec::MappedVec, token::Token, v_pool_sim::AnyPoolSim};

pub struct ChainSim {
    pub pools: MappedVec<AnyPoolSim>,
    pub tokens: MappedVec<Token>,
    pub pools_by_token: HashMap<H160,Vec<(H160,bool)>>,
}

impl ChainSim {
    pub fn new(&mut self, c_src: &ChainSrc) -> Self {
        Self { pools: todo!(), tokens: todo!(), pools_by_token: todo!() }
    }
}
