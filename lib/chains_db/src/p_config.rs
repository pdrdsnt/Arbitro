use alloy::primitives::{
    Address,
    aliases::{I24, U24},
};
use bincode::{Decode, Encode};
use sol::sol_types::PoolKey;

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum AnyPoolConfig {
    V2(V2Config),
    V3(V3Config),
    V4(V4Config),
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct V2Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
    #[bincode(with_serde)]
    pub token0: Option<Address>,
    #[bincode(with_serde)]
    pub token1: Option<Address>,
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct V3Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
    #[bincode(with_serde)]
    pub tick_spacing: Option<I24>,
    #[bincode(with_serde)]
    pub token0: Option<Address>,
    #[bincode(with_serde)]
    pub token1: Option<Address>,
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct V4Config {
    #[bincode(with_serde)]
    pub fee: U24,
    #[bincode(with_serde)]
    pub tick_spacing: I24,
    #[bincode(with_serde)]
    pub hooks: Address,
    #[bincode(with_serde)]
    pub token0: Address,
    #[bincode(with_serde)]
    pub token1: Address,
}

impl From<PoolKey> for V4Config {
    fn from(value: PoolKey) -> Self {
        Self {
            token0: value.currency0,
            token1: value.currency1,
            fee: value.fee,
            tick_spacing: value.tickSpacing,
            hooks: value.hooks,
        }
    }
}
