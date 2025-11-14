use std::collections::BTreeMap;

use alloy_primitives::{
    B256, U160,
    aliases::{I24, U24},
    keccak256,
};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use chain_db::{
    p_config::V4Config,
    p_ticks::{II24, PoolWords, TickData, TicksBitMap},
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use sol::sol_types::{PoolKey, StateView::StateViewInstance};
use v3::v3_base::bitmap_math;

use crate::clpool::CLPool;
type Slot0tuple = (U160, I24, U24, U24);

pub struct V4Data {
    pub slot0: Option<Slot0tuple>,
    pub liquidity: Option<u128>,
    pub ticks: PoolWords,
    pub pool_key: PoolKey,
}

impl V4Data {
    pub fn new(pool_key: PoolKey) -> Self {
        V4Data {
            slot0: None,
            liquidity: None,
            ticks: PoolWords {
                words: BTreeMap::new(),
            },
            pool_key,
        }
    }

    pub fn to_config(&self) -> V4Config {
        V4Config {
            fee: self.pool_key.fee,
            tick_spacing: self.pool_key.tickSpacing,
            hooks: self.pool_key.hooks,
            token0: self.pool_key.currency0,
            token1: self.pool_key.currency1,
        }
    }
}

pub struct V4Pool<P: Provider + Clone> {
    pub contract: StateViewInstance<P>,
    pub data: V4Data,
    pub id: B256,
}

impl<P: Provider + Clone> V4Pool<P> {
    pub fn new(contract: StateViewInstance<P>, data: V4Data) -> Self {
        let id = keccak256(data.pool_key.abi_encode());
        V4Pool { contract, data, id }
    }

    pub async fn sync_slot0(&mut self) -> Result<Slot0tuple, alloy::contract::Error> {
        let slot = self.contract.getSlot0(self.id).call().await?;
        let s0 = Slot0tuple::from((slot.sqrtPriceX96, slot.tick, slot.protocolFee, slot.lpFee));
        self.data.slot0 = Some(s0);
        Ok(s0)
    }
    pub async fn sync_liquidity(&mut self) -> Result<u128, alloy::contract::Error> {
        let liq = self.contract.getLiquidity(self.id).call().await?;
        self.data.liquidity = Some(liq);
        Ok(liq)
    }

    pub async fn sync_ticks(&mut self) -> Result<PoolWords, alloy::contract::Error> {
        let word_pos = bitmap_math::get_pos_from_tick(
            self.data.slot0.clone().unwrap().1,
            self.data.pool_key.tickSpacing,
        );

        let bm = self.bitmap_call(word_pos).await?;
        let ticks_from_bitmap =
            bitmap_math::extract_ticks_from_bitmap(bm, word_pos, self.data.pool_key.tickSpacing);

        let mut futs = Vec::new();
        for tick in ticks_from_bitmap.iter() {
            futs.push(self.tick_call(*tick));
        }

        let ticks_data: Vec<Result<i128, alloy::contract::Error>> = join_all(futs).await;
        let mut ticks_map = BTreeMap::new();

        for (t, d) in ticks_from_bitmap.into_iter().zip(ticks_data.into_iter()) {
            let tk: II24 = t.into();
            let dt = TickData {
                liquidity_net: d.ok(),
            };

            ticks_map.insert(tk, dt);
        }

        self.data.ticks.words.insert(
            word_pos,
            TicksBitMap {
                bitmap: bm,
                ticks: ticks_map,
            },
        );

        Ok(self.data.ticks.clone())
    }
}

impl<P: Provider + Clone> CLPool for V4Pool<P> {
    async fn bitmap_call(
        &self,
        word: i16,
    ) -> Result<alloy_primitives::Uint<256, 4>, alloy::contract::Error> {
        let bitmap = self.contract.getTickBitmap(self.id, word).call().await?;
        Ok(bitmap)
    }

    async fn tick_call(&self, tick: I24) -> Result<i128, alloy::contract::Error> {
        let tick_data = self.contract.getTickInfo(self.id, tick).call().await?;
        Ok(tick_data.liquidityNet)
    }
}
