use std::{collections::HashMap, hash::Hash};

use ethers::types::H160;
use ethers_providers::{Provider, Ws};

use crate::{arbitro::Arbitro, v_pool_sim::AnyPoolSim};

pub struct Simulacrum<A, K: Hash, V>
where K: Eq + Hash {
    origin: A,
    modifications: HashMap<K, Vec<StateTracker<V>>>,
}

impl<A, K, V> Simulacrum<A, K, V>
where K: Eq + Hash
{
    pub fn new(origin:A) -> Self {
        Self { origin, modifications: HashMap::new() }
    }

    pub fn modify(&mut self, key: K, tracker: StateTracker<V>) { 
        self.modifications.entry(key).or_insert_with(|| Vec::new()).push(tracker);
    }
    pub fn get(&self, key: &K) -> Option<&Vec<StateTracker<V>>> { 
        self.modifications.get(key)
    }

    /// ✅ Get **immutable** reference to origin
    pub fn origin(&self) -> &A {
        &self.origin
    }

    /// ✅ Get **mutable** reference to origin
    pub fn origin_mut(&mut self) -> &mut A {
        &mut self.origin
    }

}

pub struct StateTracker<V> {
    id: u128,
    updated: V,
}
