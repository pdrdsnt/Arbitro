use std::str::FromStr;
use ethers::{abi::Address, types::H160};


pub struct Pair {
    pub a: Address,
    pub b: Address,
}

impl Into<String> for Pair {
    fn into(self) -> String{
        let mut arr = [self.a.to_string(),self.b.to_string()];
        arr.sort();
        arr.join("").to_string()
    }
}

impl TryFrom<[String; 2]> for Pair {
    type Error = String;

    fn try_from(value: [String; 2]) -> Result<Self, Self::Error> {

        if value[0].len() != 42 || value[1].len() != 42 {
            return Err(("").to_string());
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
