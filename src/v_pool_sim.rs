use ethers::types::{H160, U256};

use crate::{trade::Trade, v2_pool_sim::V2PoolSim, v3_pool_sim::V3PoolSim};

#[derive(Debug,)]
pub enum AnyPoolSim {
    V2(V2PoolSim,),
    V3(V3PoolSim,),
}

impl AnyPoolSim {
    /// Purely synchronous AMM calculation, mutating local reserves
    pub fn trade(&mut self, amount_in: U256, from0: bool,) -> Option<Trade,> {
        match self {
            AnyPoolSim::V2(sim,) => sim.trade(amount_in, from0,),
            AnyPoolSim::V3(sim,) => sim.trade(amount_in, from0,),
        }
    }

    pub fn get_tokens(&self,) -> [H160; 2] {
        match self {
            AnyPoolSim::V2(v2_pool,) => [v2_pool.token0.address, v2_pool.token1.address,],
            AnyPoolSim::V3(v3_pool,) => [v3_pool.token0.address, v3_pool.token1.address,],
        }
    }
    pub fn get_address(&self,) -> H160 {
        match self {
            AnyPoolSim::V2(v2_pool,) => v2_pool.address,
            AnyPoolSim::V3(v3_pool,) => v3_pool.address,
        }
    }
    pub fn is_0(&self, token: &H160,) -> bool {
        match self {
            AnyPoolSim::V2(v2_pool,) => (v2_pool.token0.address == *token),
            AnyPoolSim::V3(v3_pool,) => (v3_pool.token0.address == *token),
        }
    }

    pub fn apply_swap(&mut self, amount0_in: U256, amount1_in: U256, amount0_out: U256, amount1_out: U256) {
        match self {
            AnyPoolSim::V2(v2_pool_sim,) => v2_pool_sim.apply_swap(amount0_in, amount1_in, amount0_out, amount1_out,),
            AnyPoolSim::V3(v3_pool_sim,) =>
            // V3 only needs the amount_in and a direction flag (from0)
            {
                if !amount0_in.is_zero() {
                    v3_pool_sim.trade(amount0_in, true,);
                } else {
                    v3_pool_sim.trade(amount1_in, false,);
                }
            },
        }
    }

    pub fn apply_mint(
        &mut self, tick_lower: Option<i32,>, tick_upper: Option<i32,>, liquidity: Option<i128,>,
        amount0: Option<U256,>, amount1: Option<U256,>,
    ) {
        match self {
            AnyPoolSim::V2(v2,) => {
                let a0 = amount0.unwrap_or_default();
                let a1 = amount1.unwrap_or_default();
                v2.mint(a0, a1,);
            },
            AnyPoolSim::V3(v3,) => {
                let lo = tick_lower.expect("tick_lower required for V3 mint",);
                let hi = tick_upper.expect("tick_upper required for V3 mint",);
                let liq = liquidity.expect("liquidity required for V3 mint",);
                v3.mint(lo, hi, liq,);
            },
        }
    }

    pub fn apply_burn(
        &mut self, tick_lower: Option<i32,>, tick_upper: Option<i32,>, liquidity: Option<i128,>,
        amount0: Option<U256,>, amount1: Option<U256,>,
    ) {
        match self {
            AnyPoolSim::V2(v2,) => {
                let a0 = amount0.unwrap_or_default();
                let a1 = amount1.unwrap_or_default();
                v2.burn(a0, a1,);
            },
            AnyPoolSim::V3(v3,) => {
                let lo = tick_lower.expect("tick_lower required for V3 burn",);
                let hi = tick_upper.expect("tick_upper required for V3 burn",);
                let liq = liquidity.expect("liquidity required for V3 burn",);
                v3.burn(lo, hi, liq,);
            },
        }
    }
}
