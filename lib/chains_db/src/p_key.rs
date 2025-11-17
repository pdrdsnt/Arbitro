use std::collections::HashSet;

use alloy::primitives::{Address, B256};
use bincode::{Decode, Encode};

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SledPoolKey {
    V2(AddId),
    V3(AddId),
    V4(AddId, #[bincode(with_serde)] B256),
}

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddId {
    id: u64,
    #[bincode(with_serde)]
    addr: Address,
}

impl AddId {
    pub fn new(id: u64, addr: Address) -> Self {
        Self { id, addr }
    }
}

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SledPairKey {
    pub id: u64,
    pub pair: Pair,
}

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pair {
    #[bincode(with_serde)]
    pub a: Address,
    #[bincode(with_serde)]
    pub b: Address,
}

impl SledPairKey {
    pub fn new(id: u64, a: Address, b: Address) -> Self {
        Self {
            id,
            pair: Pair { a, b },
        }
    }
}

#[derive(Decode, Encode, Debug)]
pub struct SledPoolCollection {
    pub inner: HashSet<SledPoolKey>,
}
