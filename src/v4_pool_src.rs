use std::sync::Arc;

use ethers::{
    abi::Address,
    contract::Contract,
    types::{H160, U256},
};
use ethers_providers::Provider;
use tokio::sync::RwLock;

use crate::{
    err::PoolUpdateError,
    mult_provider::MultiProvider,
    tick_math::{self, Tick},
    token::Token,
    v3_pool_sim::V3PoolSim, // ← your V3 sim types
};

/// A “source” for a Uniswap v4 pool.  
/// Internally it calls the single V4 factory/vault contract to
/// fetch *any* pool’s state by parameters, then feeds that
/// into your existing V3 simulation.
pub struct V4PoolSrc {
    /// The singleton V4 “factory” or “vault” contract
    factory: Contract<Provider<MultiProvider,>,>,

    /// The three parameters that identify a pool in V4:
    token0: Arc<RwLock<Token,>,>,
    token1: Arc<RwLock<Token,>,>,
    fee: u32,
    tick_spacing: i32,

    /// Last‐fetched on-chain state:
    pub sqrt_price_x96: U256,
    pub liquidity: U256,
    pub active_ticks: Vec<Tick,>,
}

impl V4PoolSrc {
    /// Build a new wrapper around the V4 factory.
    pub fn new(
        factory: Contract<Provider<MultiProvider,>,>, token0: Arc<RwLock<Token,>,>, token1: Arc<RwLock<Token,>,>,
        fee: u32, tick_spacing: i32,
    ) -> Self {
        Self {
            factory,
            token0,
            token1,
            fee,
            tick_spacing,
            sqrt_price_x96: U256::zero(),
            liquidity: U256::zero(),
            active_ticks: Vec::new(),
        }
    }

    /// Fetch the on-chain state into `self.sqrt_price_x96`, `self.liquidity`, `self.active_ticks`.
    ///  
    /// In V4 there is no pool contract; instead the factory/vault exposes:
    /// 1. `getPool`
    /// 2. `observe` / `snapshotCumulativesInside` or similar to get tick data
    pub async fn update(&mut self,) -> Result<(), PoolUpdateError,> {
        // 1) Call factory.getPool(token0, token1, fee) → returns pool ID (e.g. bytes32 or H160)
        let pool_id_call = self.factory.method::<(H160, H160, u32,), H160>(
            "getPool",
            (self.token0.read().await.address, self.token1.read().await.address, self.fee,),
        )?;
        let pool_addr = pool_id_call.call().await?;

        // 2) Call factory.getPoolState(pool_id) → (sqrtPriceX96, liquidity)
        let state_call = self.factory.method::<H160, (U256, U256,)>("getPoolState", pool_addr,)?.call().await?;
        let (sqrt_price, liquidity,) = state_call;
        self.sqrt_price_x96 = sqrt_price;
        self.liquidity = liquidity;

        // 3) Fetch active ticks around current tick
        //    Here we fetch e.g.  `snapshotCumulativesInside(pool_id, lower, upper)`
        //    or `factory.getPopulatedTicks(pool_id, startIndex, endIndex)`
        //    depending on V4 interface.  Suppose we have:
        //
        //    fn getTicks(pool: H160, start: i32, end: i32) -> (Vec<i32>, Vec<i128>);
        //
        //    which returns parallel arrays of tick index & net liquidity.
        let current_tick = tick_math::tick_from_price(self.sqrt_price_x96,)
            .ok_or(PoolUpdateError::Custom("invalid price".to_string(),),)?;

        let window = 10 * self.tick_spacing; // e.g. ±10 ticks
        let ticks_call = self
            .factory
            .method::<(H160, i32, i32,), (Vec<i32,>, Vec<i128,>,)>(
                "getTicks",
                (pool_addr, current_tick - window, current_tick + window,),
            )?
            .call()
            .await?;

        // transform into your Tick struct
        let (idxs, nets,) = ticks_call;
        self.active_ticks = idxs
            .into_iter()
            .zip(nets.into_iter(),)
            .map(|(tick, net,)| Tick {
                tick,
                liquidityNet: net,
            },)
            .collect();

        Ok((),)
    }

    /// Convert into your existing V3 simulation type
    pub async fn into_sim(&self,) -> V3PoolSim {
        V3PoolSim {
            exchange: "UniswapV4".into(),
            version: "v4".into(),
            fee: self.fee,
            tick_spacing: self.tick_spacing,
            x96price: self.sqrt_price_x96,
            liquidity: self.liquidity,
            active_ticks: self.active_ticks.clone(),
            address: todo!(),
            token0: todo!(),
            token1: todo!(),
        }
    }
}
