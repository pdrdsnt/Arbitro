use alloy_primitives::{aliases::I24, ruint::aliases::U256};

use super::ticks::Ticks;

#[derive(Debug, Clone)]
pub struct V3State {
    pub tick: I24,
    pub ticks: Ticks,
    pub liquidity: U256,
    pub x96price: U256,
}
