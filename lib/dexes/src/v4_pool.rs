use alloy_primitives::{
    B256, U160,
    aliases::{I24, U24},
    keccak256,
};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use sol::sol_types::{PoolKey, StateView::StateViewInstance};
use v3::v3_base::bitmap_math;
type Slot0tuple = (U160, I24, U24, U24);

use crate::{clpool::CLPool, words::PoolWords};
#[derive(Serialize, Deserialize, Clone)]
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
            ticks: PoolWords::new(),
            pool_key,
        }
    }
}

pub struct V4Pool<P: Provider + Clone> {
    pub contract: StateViewInstance<P>,
    pub data: V4Data,

    pub id: B256,
}

impl<P: Provider + Clone> V4Pool<P> {
    pub async fn try_fill_pool_rpc(mut self) -> Self {
        let contract = &self.contract;
        let a = self.data.pool_key.abi_encode();
        let id = keccak256(a);
        self.id = id;

        if self.data.slot0.is_none() {
            if let Ok(slot) = contract.getSlot0(id).call().await {
                self.data.slot0 =
                    Some((slot.sqrtPriceX96, slot.tick, slot.protocolFee, slot.lpFee));
            }
        }

        if self.data.liquidity.is_none() {
            if let Ok(liq) = contract.getLiquidity(id).call().await {
                self.data.liquidity = Some(liq);
            }
        }

        let key = bitmap_math::get_pos_from_tick(
            self.data.slot0.clone().unwrap().1,
            self.data.pool_key.tickSpacing,
        );
        if let Some(ticks) = self
            .get_word_ticks(key, self.data.pool_key.tickSpacing)
            .await
        {
            self.data.ticks.insert(key, ticks);
        }
        self
    }
}
