use std::{collections::HashMap, hash::Hash, rc::Rc, str::FromStr};

use ethers::{core::k256::U256, types::H160};
use ethers_providers::{Provider, Ws};

use crate::{arbitro::{self, Arbitro}, pool_action::PoolAction, trade::Trade, v_pool_sim::AnyPoolSim};

pub struct Simulacrum
{
    origin: Arbitro,
    state_tracker: HashMap<H160, Vec<PhantonState>>,
    anchor_nodes: Vec<H160>,
    stored_trades: HashMap<H160,Vec<Trade>>,
    stored_anchor_trades: HashMap<H160,Vec<Trade>>

}

impl Simulacrum
{
    pub fn new(origin:Arbitro) -> Self {
        Self {
            origin, 
            state_tracker: HashMap::new(),      
            anchor_nodes: vec![
                H160::from_str("0x55d398326f99059fF775485246999027B3197955").unwrap(),
                H160::from_str("0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d").unwrap()
            ], //stable coins to immediate path evaluation
            stored_trades:  HashMap::new(),
            stored_anchor_trades:  HashMap::new(),
        }
    }


    pub fn update_phanton_state(&mut self, addr: &H160, action: PoolAction){
        if let Some(pool_fork) = self.get(addr) {
            print!("lalalala");
        }
        
    }

    pub fn update_main_state(&mut self, addr: &H160, action: PoolAction){
        self.origin.update_state(&addr, action);
    }

    pub fn modify(&mut self, key: H160, tracker: PhantonState) { 
        self.state_tracker.entry(key).or_insert_with(|| Vec::new()).push(tracker);
    }
    pub fn get(&self, key: &H160) -> Option<&Vec<PhantonState>> { 
        self.state_tracker.get(key)
    }

}

pub struct PhantonState {
    block: u128,
    updated: AnyPoolSim,
    anchor: HashMap<H160,Vec<Trade>>,
    trades: HashMap<H160,Vec<Trade>>
}

pub struct Path {
    trades: Vec<Trade>,
    start_amount: U256,
    anchor_best: U256,
    amount_out: U256,
}