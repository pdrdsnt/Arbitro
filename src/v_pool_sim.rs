use ethers::types::{H160, U256};

use crate::{trade::Trade, v2_pool_sim::V2PoolSim, v3_pool_sim::V3PoolSim};


#[derive(Debug)]
pub enum AnyPoolSim {
    V2(V2PoolSim),
    V3(V3PoolSim),
}

impl AnyPoolSim {
    /// Purely synchronous AMM calculation, mutating local reserves
    pub fn trade(&mut self, amount_in: U256, from0: bool) -> Option<Trade> {
        match self {
            AnyPoolSim::V2(sim) => sim.trade(amount_in, from0),
            AnyPoolSim::V3(sim) => sim.trade(amount_in, from0),
        }
    }

    pub fn get_tokens(&self) -> [H160; 2] {
        match self {
            AnyPoolSim::V2(v2_pool) => [
                v2_pool.token0.address,
                v2_pool.token1.address,
            ],
            AnyPoolSim::V3(v3_pool) => [
                v3_pool.token0.address,
                v3_pool.token1.address,
            ],
        }
    }
    pub fn get_address(&self) -> H160 {
        match self {
            AnyPoolSim::V2(v2_pool) => v2_pool.address,
            AnyPoolSim::V3(v3_pool) => v3_pool.address,
        }
    }
    pub fn is_0(&self, token: &H160) -> bool {
        match self {
            AnyPoolSim::V2(v2_pool) => (v2_pool.token0.address == *token),
            AnyPoolSim::V3(v3_pool) => (v3_pool.token0.address == *token),
        }
    }
}
