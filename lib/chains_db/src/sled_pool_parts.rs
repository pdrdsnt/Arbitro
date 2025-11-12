use std::collections::BTreeMap;

use alloy::primitives::Address;
use alloy::primitives::U160;
use alloy::primitives::U256;
use alloy::primitives::aliases::I24;
use alloy::primitives::aliases::U24;
use alloy::primitives::keccak256;
use alloy::providers::Provider;
use alloy::sol_types::SolValue;
use bincode::Decode;
use bincode::Encode;
use sol::sol_types::IUniswapV2Pair::IUniswapV2PairInstance;
use sol::sol_types::PoolKey;
use sol::sol_types::StateView::StateViewInstance;
use sol::sol_types::V3Pool::V3PoolInstance;

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolTokens {
    #[bincode(with_serde)]
    pub a: Option<Address>,
    #[bincode(with_serde)]
    pub b: Option<Address>,
}

#[derive(Decode, Encode, Debug)]
pub enum AnyPoolState {
    V2(V2PoolState),
    V3(V3PoolState),
}

#[derive(Decode, Encode, Debug)]
pub struct V3PoolState {
    #[bincode(with_serde)]
    pub tick: Option<I24>,
    #[bincode(with_serde)]
    pub x96price: Option<U160>,
    #[bincode(with_serde)]
    pub liquidity: Option<u128>,
}

#[derive(Decode, Encode, Debug)]
pub struct V2PoolState {
    #[bincode(with_serde)]
    pub r0: u128,
    #[bincode(with_serde)]
    pub r1: u128,
}

#[derive(Decode, Encode, Debug)]
pub struct PoolWord {
    pub ticks: Vec<PoolTick>,
}

#[derive(Decode, Encode, Debug)]
pub struct PoolWords {
    pub words: BTreeMap<i16, PoolWord>,
}

#[derive(Decode, Encode, Debug)]
pub struct PoolTick {
    #[bincode(with_serde)]
    pub tick: I24,

    #[bincode(with_serde)]
    pub liquidity_net: Option<i128>,
}

#[derive(Decode, Encode, Debug)]
pub enum AnyPoolSled {
    V2(
        u64,
        #[bincode(with_serde)] Address,
        V2Config,
        V2PoolState,
        PoolTokens,
    ),
    V3(
        u64,
        #[bincode(with_serde)] Address,
        V3Config,
        V3PoolState,
        AnyPoolLiquidityNets,
        PoolTokens,
    ),
    V4(
        u64,
        #[bincode(with_serde)] Address,
        V4Config,
        V3PoolState,
        AnyPoolLiquidityNets,
        PoolTokens,
    ),
}
#[derive(Decode, Encode, Debug)]
pub enum AnyPoolConfig {
    V2(V2Config),
    V3(V3Config),
    V4(V4Config),
}

#[derive(Decode, Encode, Debug)]
pub struct V2Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
}

#[derive(Decode, Encode, Debug)]
pub struct V3Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
    #[bincode(with_serde)]
    pub tick_spacing: Option<I24>,
}

#[derive(Decode, Encode, Debug)]
pub struct V4Config {
    #[bincode(with_serde)]
    pub fee: U24,
    #[bincode(with_serde)]
    pub tick_spacing: I24,
    #[bincode(with_serde)]
    pub hooks: Address,
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WordPos(#[bincode(with_serde)] I24);
impl Into<I24> for WordPos {
    fn into(self) -> I24 {
        self.0
    }
}
impl From<I24> for WordPos {
    fn from(value: I24) -> Self {
        Self(value)
    }
}
#[derive(Decode, Encode, Debug)]
pub struct AnyPoolLiquidityNets {
    pub ticks: BTreeMap<WordPos, TicksBitMap>,
}
#[derive(Decode, Encode, Debug)]
pub struct TicksBitMap {
    #[bincode(with_serde)]
    pub bitmap: U256,
    pub ticks: BTreeMap<WordPos, TickData>,
}

#[derive(Encode, Decode, Debug)]
pub struct TickData {
    #[bincode(with_serde)]
    pub liquidity_net: Option<i128>,
}
