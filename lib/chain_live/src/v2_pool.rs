use alloy_primitives::{Address, aliases::U24};
use alloy_provider::Provider;
use chain_db::sled_pool_parts::PoolTokens;
use serde::{Deserialize, Serialize};
use sol::sol_types::IUniswapV2Pair::IUniswapV2PairInstance;

#[derive(Clone, Serialize, Deserialize)]
pub struct V2Data {
    pub name: Option<String>,
    pub reserves: Option<(u128, u128)>,
    pub fee: Option<U24>,
    pub token0: Option<Address>,
    pub token1: Option<Address>,
}

impl V2Data {
    pub async fn new_with(fee: U24, pair: PoolTokens) -> V2Data {
        Self {
            name: None,
            reserves: None,
            fee: Some(fee),
            token0: pair.a,
            token1: pair.b,
        }
    }
}

pub struct V2Pool<P: Provider + Clone> {
    pub contract: IUniswapV2PairInstance<P>,
    pub data: V2Data,
}

impl<P: Provider + Clone> V2Pool<P> {
    pub fn new(contract: IUniswapV2PairInstance<P>, data: V2Data) -> V2Pool<P> {
        Self { contract, data }
    }

    pub async fn try_fill_rpc(mut self) -> Self {
        let contract = &self.contract;
        if self.data.name.is_none() {
            if let Ok(name) = contract.name().call().await {
                self.data.name = Some(name);
            }
        }

        if self.data.reserves.is_none() {
            if let Ok(r) = contract.getReserves().call().await {
                self.data.reserves = Some((r.reserve0.to(), r.reserve1.to()));
            };
        }
        if self.data.token0.is_none() {
            if let Ok(t0) = contract.token0().call().await {
                self.data.token0 = Some(t0);
            };
        }
        if self.data.token0.is_none() {
            if let Ok(t1) = contract.token1().call().await {
                self.data.token1 = Some(t1);
            };
        }

        self
    }
}
