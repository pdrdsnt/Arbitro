use std::{hash::{Hash, Hasher}};
use ethers::types::H160;
use crate::token::Token;

#[derive(Clone, Debug)]
pub struct Pair {
    pub a: Token,
    pub b: Token,
}

impl Pair {
    pub fn new(token1: Token, token2: Token) -> Self {
        if token1.address < token2.address {
            Pair { a: token1, b: token2 }
        } else {
            Pair { a: token2, b: token1 }
        }
    }
}

impl PartialEq for Pair {
    fn eq(&self, other: &Self) -> bool {
        self.a.address == other.a.address && self.b.address == other.b.address
    }
}

impl Eq for Pair {}

impl Hash for Pair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.address.hash(state);
        self.b.address.hash(state);
    }
}

impl From<Pair> for String {
    fn from(val: Pair) -> Self {
        format!("{}{}", val.a.address, val.b.address)
    }
}
