use std::{
    collections::{BinaryHeap, HashMap, VecDeque},
    str::FromStr,
};

use ethers::types::{H160, U256};

use crate::{
    chain_src::ChainSrc, mapped_vec::MappedVec, pool_action::PoolAction, token::Token,
    trade::Trade, v3_pool_sim::V3PoolSim, v_pool_sim::AnyPoolSim,
};

pub struct Arbitro {
    pub pools: MappedVec<AnyPoolSim>,
    pub pools_by_token: HashMap<H160, Vec<(H160, bool)>>,
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

        Self { pools, pools_by_token }
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
            } => {
                self.pools.get_mut(addr).unwrap().apply_mint(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );
            }
            PoolAction::BurnV3 { owner, tick_lower, tick_upper, amount, amount0, amount1 } => {
                self.pools.get_mut(addr).unwrap().apply_burn(
                    None,
                    None,
                    None,
                    Some(amount0),
                    Some(amount1),
                );
            }
        }
    }

    pub fn evaluate_hop(
        &mut self, from: &H160, pools: Vec<H160>, phanton: Vec<H160>, amount: U256,
    ) -> BinaryHeap<Trade> {
        let mut trades_queue = BinaryHeap::<Trade>::new();
        for pool_addr in pools {
            if let Some(pool) = self.pools.get_mut(&pool_addr) {
                let is0 = pool.is_0(from);
                if let Some(trade) = pool.trade(amount, is0) {
                    trades_queue.push(trade);
                }
            }
        }

        trades_queue
    }

    pub fn lazy_propagate(&mut self, from: &H160) -> HashMap<H160, Vec<H160>> {
        let mut paths_by_pair = HashMap::<H160, Vec<H160>>::new();
        for pools in self.pools_by_token.get(from) {
            for (addr, is0) in pools {
                if let Some(pool) = self.pools.get(&addr) {
                    if let Some(other) = pool.get_tokens().into_iter().find(|z| z != addr) {
                        paths_by_pair.entry(*from).or_insert(Vec::new()).push(other);
                    }
                }
            }
        }

        paths_by_pair
    }

    pub fn add_token(&mut self, token_addr: H160) {
        self.pools_by_token.entry(token_addr).or_insert_with(Vec::new);
    }

    /// 2) Register a newly discovered pool‐simulation at runtime.
    ///
    ///    - `pool_addr`: the on‐chain address of the pool
    ///    - `pool_sim`: the `AnyPoolSim` (simulation object) for that pool
    ///
    ///    This method:
    ///      • Inserts (`pool_addr` ↦ `pool_sim`) into `self.pools`.
    ///      • Extracts `(token0, token1) = pool_sim.get_tokens()`.
    ///      • Updates both `pools_by_token[token0]` and `pools_by_token[token1]`
    ///        with `(pool_addr, is_0)`.
    ///      • Updates `tokens_by_pool[pool_addr] = (token0, token1)`.
    pub fn add_pool(&mut self, pools_sim: AnyPoolSim) {
        // 1) Insert into `self.pools`

            // 2) Figure out the two tokens in this pool:
            let tokens = pools_sim.get_tokens();
            let token0: H160 = tokens[0];
            let token1 = tokens[1];

            // 4) Update pools_by_token[token0] with (pool_addr, is_0 = true)
            self.pools_by_token.entry(token0).or_insert_with(Vec::new).push((pools_sim.get_address(), true));

            // 5) Update pools_by_token[token1] with (pool_addr, is_0 = false)
            self.pools_by_token.entry(token1).or_insert_with(Vec::new).push((pools_sim.get_address(), false));
        
    }
}
