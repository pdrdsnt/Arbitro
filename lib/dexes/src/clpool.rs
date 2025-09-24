use alloy_primitives::aliases::I24;
use alloy_provider::Provider;
use futures::future::join_all;
use v3::v3_base::bitmap_math;

use crate::{
    v3_pool::V3Pool,
    v4_pool::V4Pool,
    words::{TickData, TicksBitMap},
};

pub trait CLPool {
    async fn get_word_ticks(&self, word: i16, tick_spacing: I24) -> Option<TicksBitMap>;

    async fn fetch_ticks(&self, ticks: Vec<I24>) -> Vec<TickData>;
}

impl<P: Provider + Clone> CLPool for V3Pool<P> {
    async fn get_word_ticks(&self, word: i16, tick_spacing: I24) -> Option<TicksBitMap> {
        if let Ok(bitmap) = self.contract.tickBitmap(word).call().await {
            let tks = bitmap_math::extract_ticks_from_bitmap(bitmap, word, tick_spacing);

            let ticks = self.fetch_ticks(tks).await;

            return Some(TicksBitMap { bitmap, ticks });
        };

        None
    }

    async fn fetch_ticks(&self, ticks: Vec<I24>) -> Vec<TickData> {
        let mut ticks_call = Vec::with_capacity(ticks.len());

        for tick in ticks.into_iter() {
            let co = self.contract.clone();
            ticks_call.push(async move { (co.ticks(tick).call().into_future().await, tick) });
        }

        let ticks: Vec<TickData> = join_all(ticks_call)
            .await
            .into_iter()
            .filter_map(|(s, i)| match s {
                Ok(tick_data) => Some(TickData {
                    tick: i,
                    liquidity_net: Some(tick_data.liquidityNet),
                }),
                Err(_) => Some(TickData {
                    tick: i,
                    liquidity_net: None,
                }),
            })
            .collect();

        return ticks;
    }
}

impl<P: Provider + Clone> CLPool for V4Pool<P> {
    async fn get_word_ticks(&self, word: i16, tick_spacing: I24) -> Option<TicksBitMap> {
        if let Ok(bitmap) = self.contract.getTickBitmap(self.id, word).call().await {
            let tks = bitmap_math::extract_ticks_from_bitmap(bitmap, word, tick_spacing);

            let ticks = self.fetch_ticks(tks).await;

            return Some(TicksBitMap { bitmap, ticks });
        };

        None
    }

    async fn fetch_ticks(&self, ticks: Vec<I24>) -> Vec<TickData> {
        let mut ticks_call = Vec::with_capacity(ticks.len());

        for tick in ticks.into_iter() {
            let co = self.contract.clone();
            ticks_call.push(async move {
                (
                    co.getTickInfo(self.id, tick).call().into_future().await,
                    tick,
                )
            });
        }

        let ticks: Vec<TickData> = join_all(ticks_call)
            .await
            .into_iter()
            .filter_map(|(s, i)| match s {
                Ok(tick_data) => Some(TickData {
                    tick: i,
                    liquidity_net: Some(tick_data.liquidityNet),
                }),
                Err(_) => Some(TickData {
                    tick: i,
                    liquidity_net: None,
                }),
            })
            .collect();

        return ticks;
    }
}
