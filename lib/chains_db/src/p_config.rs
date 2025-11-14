use alloy::primitives::{
    Address,
    aliases::{I24, U24},
};
use bincode::{Decode, Encode};

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyPoolConfig {
    V2(V2Config),
    V3(V3Config),
    V4(V4Config),
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct V2Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
    #[bincode(with_serde)]
    pub token0: Option<Address>,
    #[bincode(with_serde)]
    pub token1: Option<Address>,
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
