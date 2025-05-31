use std::collections::HashMap;

use ethers::types::{H160, U256};

use crate::{
    chain_src::ChainSrc, mapped_vec::MappedVec, pool_action::PoolAction, token::Token,
    v3_pool_sim::V3PoolSim, v_pool_sim::AnyPoolSim,
};

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

        Self { pools, pools_by_token, tokens_by_pool }
    }

    pub fn update_state(&mut self, addr: &H160, action: PoolAction) {
        match action {
            PoolAction::SwapV2 { amount0_in, amount1_in, sender, amount0_out, amount1_out, to } => {
                self.pools.get_mut(addr).unwrap().apply_swap(
                    amount0_in,
                    amount1_in,
                    amount0_out,
                    amount1_out,
                );
            }
            PoolAction::MintV2 { amount0, amount1, sender } => {
                self.pools.get_mut(addr).unwrap().apply_mint(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );
            }
            PoolAction::BurnV2 { amount0, amount1, sender, to } => {
                self.pools.get_mut(addr).unwrap().apply_burn(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );
            }
            PoolAction::SwapV3 {
                sender,
                recipient,
                amount0,
                amount1,
                sqrt_price_x96,
                liquidity,
                tick,
            } => {
                self.pools.get_mut(addr).unwrap().apply_swap(
                    U256::from(amount0),
                    U256::from(amount1),
                    U256::from(0),
                    U256::from(0),
                );
            }
            PoolAction::MintV3 {
                sender,
                owner,
                tick_lower,
                tick_upper,
                amount,
                amount0,
                amount1,
            } => {self.pools.get_mut(addr).unwrap().apply_mint(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );},
            PoolAction::BurnV3 { owner, tick_lower, tick_upper, amount, amount0, amount1 } =>
                {self.pools.get_mut(addr).unwrap().apply_burn(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );},
        }
    }
}
