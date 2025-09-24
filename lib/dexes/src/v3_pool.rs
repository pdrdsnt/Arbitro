use alloy_primitives::{
    Address, U160,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};
use sol::sol_types::V3Pool::V3PoolInstance;
use v3::v3_base::bitmap_math;

use crate::{clpool::CLPool, words::PoolWords};

type Slot0tuple = (U160, I24, u16, u16, u16, u8, bool);

#[derive(Clone, Serialize, Deserialize)]
pub struct V3Data {
    pub slot0: Option<Slot0tuple>,
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

    pub async fn try_fill_pool_rpc(mut self, contract: V3PoolInstance<P>) -> Self {
        if self.data.slot0.is_none() {
            if let Ok(slot) = contract.slot0().call().await {
                self.data.slot0 = Some((
                    slot.sqrtPriceX96,
                    slot.tick,
                    slot.observationIndex,
                    slot.observationCardinality,
                    slot.observationCardinalityNext,
                    slot.feeProtocol,
                    slot.unlocked,
                ));
            }
        }

        if self.data.liquidity.is_none() {
            if let Ok(liq) = contract.liquidity().call().await {
                self.data.liquidity = Some(liq);
            }
        }
        if self.data.fee.is_none() {
            if let Ok(fee) = contract.fee().call().await {
                self.data.fee = Some(fee);
            }
        }

        if self.data.tick_spacing.is_none() {
            if let Ok(ts) = contract.tickSpacing().call().await {
                self.data.tick_spacing = Some(ts);
            }
        }

        if self.data.token0.is_none() {
            if let Ok(t) = contract.token0().call().await {
                self.data.token0 = Some(t);
            }
        }

        if self.data.token1.is_none() {
            if let Ok(t) = contract.token1().call().await {
                self.data.token1 = Some(t);
            }
        }

        if self.data.tick_spacing.is_some() && self.data.slot0.is_some() {
            let key = bitmap_math::get_pos_from_tick(
                self.data.slot0.unwrap().1,
                self.data.tick_spacing.unwrap(),
            );

            if let Some(ticks) = self
                .get_word_ticks(key, self.data.tick_spacing.unwrap())
                .await
            {
                self.data.ticks.insert(key, ticks);
            }
        }

        self
    }
}

impl Default for V3Data {
    fn default() -> Self {
        V3Data {
            slot0: None,
            liquidity: None,
            fee: None,
            ticks: PoolWords::new(),
            tick_spacing: None,
            token0: None,
            token1: None,
        }
    }
}
