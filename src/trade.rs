use ethers::{core::types::U256, types::H160};


#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Trade {
    pub dex: String,
    pub version: String,
    pub fee: u32,
    pub token0: H160,
    pub token1: H160,
    pub pool: H160,
    pub from0: bool,
    pub amount_in: ethers::types::U256,
    pub amount_out: ethers::types::U256,
    pub price_impact: U256,
    pub fee_amount: U256,
    pub raw_price: U256,
}
