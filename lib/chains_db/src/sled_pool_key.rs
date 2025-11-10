use alloy::primitives::{Address, B256};
use bincode::{Decode, Encode};

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SledPoolKey {
    V2(AddId),
    V3(AddId),
    V4(AddId, #[bincode(with_serde)] B256),
}

impl SledPoolKey {
    pub fn enc(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }
    fn from_anykey(value: dexes::any_pool::AnyPoolKey, chain_id: u64) -> Self {
        match value {
            dexes::any_pool::AnyPoolKey::V2(address) => Self::V2(AddId {
                id: chain_id,
                addr: address,
            }),
            dexes::any_pool::AnyPoolKey::V3(address) => Self::V3(AddId {
                id: chain_id,
                addr: address,
            }),
            dexes::any_pool::AnyPoolKey::V4(address, fixed_bytes) => Self::V4(
                AddId {
                    id: chain_id,
                    addr: address,
                },
                fixed_bytes,
            ),
        }
    }
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
