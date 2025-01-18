use std::{fmt::format, hash::{Hash, Hasher}, str::FromStr};
use axum::Error;
use ethers::{abi::Address, types::H160};

#[derive(Clone,Debug)]
pub struct Pair {
    pub a: Address,
    pub b: Address,
}

impl PartialEq for Pair {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}
impl Hash for Pair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Ordena `a` e `b` antes de calcular o hash, garantindo que a ordem não importe
        let (first, second) = if self.a < self.b {
            (&self.a, &self.b)
        } else {
            (&self.b, &self.a)
        };
        first.hash(state);
        second.hash(state);
    }
}
impl Eq for Pair {}

impl From<Pair> for String {
    fn from(val: Pair) -> Self{
        let mut arr = [val.a.to_string(),val.b.to_string()];
        arr.sort();
        arr.join("").to_string()
    }
}


impl TryFrom<[String; 2]> for Pair {
    type Error = String;

    fn try_from(value: [String; 2]) -> Result<Self, Self::Error> {

        if value[0].len() != 42 || value[1].len() != 42 {
            println!("error converting, string lenght need to be 42");
            println!("lenght 0 = {}", value[0].len());
            println!("lenght 1 = {}", value[1].len());
            return Err("".to_string());
        }
        let mut addresses = value; // Create a mutable copy of the array
        // Sort the addresses in-place
        addresses.sort(); 

        let a = H160::from_str(&addresses[0]).unwrap();
        let b = H160::from_str(&addresses[1]).unwrap();
        //print!("address a: {}", a.to_string());
        //print!("address b: {}", b.to_string());
        Ok(Pair { a, b })
    }
}

impl TryFrom<String> for Pair {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {

        if value.len() != 83 {
            return Err(("").to_string());
        }
        let _a: &str = &value[0..42];
        let _b: &str = &value[42..84];

        let mut sorted= [_a,_b];
        sorted.sort();

        let a: H160 = Address::from_str(sorted[0]).unwrap();
        let b: H160 = Address::from_str(sorted[1]).unwrap();
        
        Ok(Pair { a, b })
    }
}
