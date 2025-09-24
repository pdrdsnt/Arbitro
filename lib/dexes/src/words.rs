use std::collections::BTreeMap;

use alloy_primitives::{Address, U256, aliases::I24};
use serde::{Deserialize, Serialize};

pub type PoolWords = BTreeMap<i16, TicksBitMap>;

#[derive(Clone, Serialize, Deserialize)]
pub struct TicksBitMap {
    pub bitmap: U256,
    pub ticks: Vec<TickData>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TickData {
    pub tick: I24,
    pub liquidity_net: Option<i128>,
}

pub struct Pair {
    pub a: Address,
    pub b: Address,
}
