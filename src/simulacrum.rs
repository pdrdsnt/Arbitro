use std::{collections::HashMap, hash::Hash};

use ethers::types::H160;
use ethers_providers::{Provider, Ws};

use crate::{arbitro::Arbitro, trade::Trade, v_pool_sim::AnyPoolSim};

pub struct Simulacrum
{
    origin: Arbitro,
    modifications: HashMap<H160, Vec<StateTracker>>,
}

impl Simulacrum
{
    pub fn new(origin:Arbitro) -> Self {
        Self { origin, modifications: HashMap::new() }
    }

    pub fn modify(&mut self, key: H160, tracker: StateTracker) { 
        self.modifications.entry(key).or_insert_with(|| Vec::new()).push(tracker);
    }
    pub fn get(&self, key: &H160) -> Option<&Vec<StateTracker>> { 
        self.modifications.get(key)
    }

    /// ✅ Get **immutable** reference to origin
    pub fn origin(&self) -> &Arbitro {
        &self.origin
    }

    /// ✅ Get **mutable** reference to origin
    pub fn origin_mut(&mut self) -> &mut Arbitro {
        &mut self.origin
    }

}

pub struct StateTracker {
    id: u128,
    updated: AnyPoolSim,
}

pub struct SuperPool {
    pool: AnyPoolSim,
    stored_trades: HashMap<H160,Vec<Trade>>,
}