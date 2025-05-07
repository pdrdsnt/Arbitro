use ethers::types::U256;

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
}
