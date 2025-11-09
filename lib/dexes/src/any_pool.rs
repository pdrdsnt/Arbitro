use alloy_primitives::{Address, B256, B512};
use alloy_provider::Provider;
use serde::{Deserialize, Serialize};

use crate::{
    v2_pool::{V2Data, V2Pool},
    v3_pool::{V3Data, V3Pool},
    v4_pool::{V4Data, V4Pool},
};

pub enum AnyPool<P: Provider + Clone> {
    V2(V2Pool<P>),
    V3(V3Pool<P>),
    V4(V4Pool<P>),
}

impl<P: Provider + Clone> AnyPool<P> {
    pub fn to_key(&self) -> AnyPoolKey {
        match self {
            AnyPool::V2(v2_pool) => AnyPoolKey::V2(*v2_pool.contract.address()),
            AnyPool::V3(v3_pool) => AnyPoolKey::V3(*v3_pool.contract.address()),
            AnyPool::V4(v4_pool) => AnyPoolKey::V4(*v4_pool.contract.address(), v4_pool.id),
        }
    }
}

pub enum AnyPoolData {
    V2(V2Data),
    V3(V3Data),
    V4(V4Data),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnyPoolKey {
    V2(Address),
    V3(Address),
    V4(Address, B256),
}
