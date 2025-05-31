use std::{collections::HashMap, hash::Hash};

use ethers::types::H160;
use ethers_providers::{Provider, Ws};

use crate::{arbitro::Arbitro, v_pool_sim::AnyPoolSim};

pub struct Simulacrum<'a, A, K: Hash, V> {
    origin: &'a A,
    modifications: HashMap<K, Vec<StateTracker<V>>>,
}

impl<'a, A, K, V> Simulacrum<'a, A, K, V>
where K: Hash
{
    pub fn new(origin: &'a A) -> Self {
        Self { origin, modifications: HashMap::new() }
    }
}

pub struct StateTracker<P> {
    id: u128,
    updated_pool: P,
}
