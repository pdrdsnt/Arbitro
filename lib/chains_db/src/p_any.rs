use alloy::primitives::Address;
use bincode::{Decode, Encode};

use crate::{
    p_config::{V2Config, V3Config, V4Config},
    p_state::{V2State, V3State},
    p_ticks::PoolWords,
    p_tokens::Tokens,
};

#[derive(Decode, Encode, Debug)]
pub enum AnyPoolSled {
    V2(u64, #[bincode(with_serde)] Address, V2Config, V2State),
    V3(
        u64,
        #[bincode(with_serde)] Address,
        V3Config,
        V3State,
        PoolWords,
    ),
    V4(
        u64,
        #[bincode(with_serde)] Address,
        V4Config,
        V3State,
        PoolWords,
    ),
}
