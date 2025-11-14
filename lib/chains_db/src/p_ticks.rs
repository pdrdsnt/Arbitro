use std::collections::BTreeMap;

use alloy::primitives::{U256, aliases::I24};
use bincode::{Decode, Encode};

#[derive(Decode, Default, Clone, Encode, Debug)]
pub struct PoolWords {
    pub words: BTreeMap<i16, TicksBitMap>,
}

#[derive(Decode, Encode, Debug, Clone)]
pub struct TicksBitMap {
    #[bincode(with_serde)]
    pub bitmap: U256,
    pub ticks: BTreeMap<II24, TickData>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct TickData {
    #[bincode(with_serde)]
    pub liquidity_net: Option<i128>,
}

#[derive(Clone, Decode, Encode, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct II24(#[bincode(with_serde)] I24);

impl Into<I24> for II24 {
    fn into(self) -> I24 {
        self.0
    }
}

impl From<I24> for II24 {
    fn from(value: I24) -> Self {
        Self(value)
    }
}
