use std::collections::BTreeMap;

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
use chain_db::{
    p_config::V3Config,
    p_ticks::{II24, PoolWords, TickData, TicksBitMap},
};
use futures::future::join_all;
use sol::sol_types::V3Pool::{V3PoolInstance, slot0Return};
use v3::v3_base::bitmap_math;

use crate::clpool::CLPool;

#[derive(Clone)]
pub struct V3Data {
    pub slot0: Option<slot0Return>,
    pub liquidity: Option<u128>,
    pub ticks: PoolWords,
    pub tick_spacing: Option<I24>,
    pub fee: Option<U24>,
    pub token0: Option<Address>,
    pub token1: Option<Address>,
}

pub struct V3Pool<P: Provider + Clone> {
    pub contract: V3PoolInstance<P>,
    pub data: V3Data,
}
impl<P: Provider + Clone> V3Pool<P> {
    pub fn new(contract: V3PoolInstance<P>, data: V3Data) -> Self {
        Self { contract, data }
    }
    pub async fn sync_slot0(&mut self) -> Result<slot0Return, alloy::contract::Error> {
        let slot = self.contract.slot0().call().await?;
        self.data.slot0 = Some(slot.clone());
        Ok(slot)
    }

    pub async fn sync_liquidity(&mut self) -> Result<u128, alloy::contract::Error> {
        let liq = self.contract.liquidity().call().await?;
        self.data.liquidity = Some(liq);
        Ok(liq)
    }

    pub async fn sync_ticks(&mut self) -> Result<PoolWords, alloy::contract::Error> {
        let Some(s0) = self.data.slot0.clone() else {
            return Err(alloy::contract::Error::UnknownFunction("aaa".to_string()));
        };

        let word_pos = bitmap_math::get_pos_from_tick(s0.tick, self.data.tick_spacing.unwrap());

        let bm = self.bitmap_call(word_pos).await?;

        let ticks_from_bitmap =
            bitmap_math::extract_ticks_from_bitmap(bm, word_pos, self.data.tick_spacing.unwrap());

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

impl Default for V3Data {
    fn default() -> Self {
        V3Data {
            slot0: None,
            liquidity: None,
            fee: None,
            ticks: PoolWords::default(),
            tick_spacing: None,
            token0: None,
            token1: None,
        }
    }
}

impl From<V3Config> for V3Data {
    fn from(value: V3Config) -> Self {
        Self {
            slot0: None,
            liquidity: None,
            ticks: PoolWords::default(),
            tick_spacing: value.tick_spacing,
            fee: value.fee,
            token0: value.token0,
            token1: value.token1,
        }
    }
}

impl<P: Provider + Clone> CLPool for V3Pool<P> {
    async fn bitmap_call(
        &self,
        word: i16,
    ) -> Result<alloy_primitives::Uint<256, 4>, alloy::contract::Error> {
        self.contract.tickBitmap(word).call().await
    }

    async fn tick_call(&self, tick: I24) -> Result<i128, alloy::contract::Error> {
        let c = self.contract.clone();
        let v3_res = c.ticks(tick).call().into_future().await?;
        Ok(v3_res.liquidityNet)
    }
}
