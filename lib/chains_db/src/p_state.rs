use alloy::primitives::{U160, aliases::I24};
use bincode::{Decode, Encode};

#[derive(Decode, Encode, Debug)]
pub enum AnyPoolState {
    V2(V2State),
    V3(V3State),
}

#[derive(Decode, Encode, Debug)]
pub struct V3State {
    #[bincode(with_serde)]
    pub tick: Option<I24>,
    #[bincode(with_serde)]
    pub x96price: Option<U160>,
    #[bincode(with_serde)]
    pub liquidity: Option<u128>,
}

#[derive(Decode, Encode, Debug)]
pub struct V2State {
    #[bincode(with_serde)]
    pub r0: u128,
    #[bincode(with_serde)]
    pub r1: u128,
}
