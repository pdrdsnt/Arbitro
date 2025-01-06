use ethers::{contract::Contract, providers::{Provider, Ws}};

pub struct DexData{
    pub name: String,
    pub version: String,
    pub factory: Contract<Provider<Ws>>,
}

impl DexData{
        pub fn new(
            name:String,
            version: String,
            factory: Contract<Provider<Ws>>,
        ) -> Self {
            DexData {name,version,factory}
        }
    }
