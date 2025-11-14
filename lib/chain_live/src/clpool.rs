use std::collections::BTreeMap;

use alloy_primitives::aliases::I24;
use chain_db::sled_pool_parts::{II24, TickData, TicksBitMap};
use futures::future::join_all;
use v3::v3_base::bitmap_math;

pub trait CLPool {
    async fn bitmap_call(
        &self,
        word: i16,
    ) -> Result<alloy_primitives::Uint<256, 4>, alloy::contract::Error>;

    async fn tick_call(&self, tick: I24) -> Result<i128, alloy::contract::Error>;

    async fn get_word_ticks(&self, word: i16, tick_spacing: I24) -> Option<TicksBitMap> {
        if let Ok(bitmap) = self.bitmap_call(word).await {
            let tks = bitmap_math::extract_ticks_from_bitmap(bitmap, word, tick_spacing);

            let ticks = self.fetch_ticks(tks).await;

            return Some(TicksBitMap { bitmap, ticks });
        };

        None
    }

    async fn fetch_ticks(&self, ticks: Vec<I24>) -> BTreeMap<II24, TickData> {
        let mut ticks_call = Vec::with_capacity(ticks.len());

        for tick in ticks.into_iter() {
            ticks_call.push(async move { (self.tick_call(tick).await, tick) });
        }

        let mut ticks: BTreeMap<II24, TickData> = BTreeMap::new();

        join_all(ticks_call)
            .await
            .into_iter()
            .for_each(|(s, i)| match s {
                Ok(tick_data) => {
                    Some(ticks.insert(
                        i.into(),
                        TickData {
                            liquidity_net: Some(tick_data),
                        },
                    ));
                }
                Err(_) => {
                    Some(ticks.insert(
                        i.into(),
                        TickData {
                            liquidity_net: None,
                        },
                    ));
                }
            });

        return ticks;
    }
}
