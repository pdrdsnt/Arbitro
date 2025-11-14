use alloy::primitives::Address;
use bincode::Decode;
use bincode::Encode;

#[derive(Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tokens {
    #[bincode(with_serde)]
    pub a: Option<Address>,
    #[bincode(with_serde)]
    pub b: Option<Address>,
}
