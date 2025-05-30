use std::{collections::HashMap, sync::Arc, vec};

use ethers::{
    abi::token,
    contract::{self, Contract},
    providers::{Http, Provider},
    types::H160,
};
use tokio::sync::RwLock;

use crate::{mult_provider::MultiProvider, pair::Pair, token::Token};

#[derive(Clone,)]
pub struct Factory {
    pub name: String,
    pub factory: Contract<Provider<MultiProvider,>,>,
}
impl Factory {
    pub fn new(name: String, contract: Contract<Provider<MultiProvider,>,>,) -> Self {
        Self {
            name,
            factory: contract,
        }
    }
}

#[derive(Clone,)]
pub enum AnyFactory {
    V2(Factory,),
    V3(Factory,),
}

impl AnyFactory {
    pub fn supported_fees(&self,) -> &'static [u32] {
        match self {
            AnyFactory::V2(_,) => &[3000,], // All V2 DEXes use 0.3% fee
            AnyFactory::V3(dex,) => match dex.name.as_str() {
                // Major DEXs
                "pancake" => &[500, 2500, 10000,],   // PancakeSwap V3
                "uniswap" => &[500, 3000, 10000,],   // Uniswap V3
                "mdex" => &[500, 2000, 10000,],      // MDEX V3
                "apeswap" => &[500, 2500, 5000,],    // ApeSwap V3
                "biswap" => &[1000, 2000, 3000,],    // Biswap V3
                "sushi" => &[500, 3000, 10000,],     // SushiSwap V3
                "wault" => &[500, 1500, 3000,],      // Wault V3
                "cheeseswap" => &[800, 2000, 5000,], // CheeseSwap V3

                // Stablecoin-focused DEXs
                "ellipsis" => &[100, 500, 2000,], // Ellipsis V3
                "belt" => &[100, 400, 1500,],     // Belt Finance V3

                // Specialized DEXs
                "alpaca" => &[500, 2000, 5000,],   // Alpaca Finance V3
                "babyswap" => &[500, 1500, 3000,], // BabySwap V3

                _ => &[], // Unknown DEX
            },
        }
    }

    pub fn new(name: String, v2: bool, factory: Contract<Provider<MultiProvider,>,>,) -> Self {
        let dex = Factory { name, factory, };

        if v2 {
            Self::V2(dex,)
        } else {
            Self::V3(dex,)
        }
    }

    pub async fn get_pool(&self, token0: &H160, token1: &H160, fee: &u32, tick_spacing: &u32,) -> Option<H160,> {
        if !self.supported_fees().contains(&fee,) {
            println!("Fee {} não suportada para {}", fee, self.get_name());
            return None;
        }
        let a = token0;
        let b = token1;

        match self {
            AnyFactory::V2(dex,) => {
                let method = dex.factory.method::<(H160, H160,), H160>("getPair", (*a, *b,),).unwrap();

                let address =
                    dex.factory.method::<(H160, H160,), H160>("getPair", (*a, *b,),).unwrap().call_raw().await.ok()?;
                if address == H160::zero() {
                    println!("No pool, returned {}", address);
                    return None;
                }

                let pool_contract = Contract::new(address, dex.factory.abi().clone(), dex.factory.client().clone(),);
                if address == H160::zero() {
                    println!("No pool, returned {}", address);
                    return None;
                }

                let dex_name = dex.name.clone();
                println!("New pool created, returned {}", address);
                Some(address,)
            },
            AnyFactory::V3(dex,) => {
                let address = dex
                    .factory
                    .method::<(H160, H160, u32,), H160>("getPool", (*a, *b, *fee,),)
                    .unwrap()
                    .call_raw()
                    .await
                    .ok()?;
                if address == H160::zero() {
                    println!("No pool, returned {}", address);
                    return None;
                }

                let pool_contract = Contract::new(address, dex.factory.abi().clone(), dex.factory.client().clone(),);

                println!("New pool created, returned {}", address);

                println!("Pool address: {}", address);
                Some(address,)
            },
        }
    }

    pub fn get_name(&self,) -> &str {
        match self {
            AnyFactory::V2(dex,) | AnyFactory::V3(dex,) => &dex.name,
        }
    }

    pub fn get_version(&self,) -> &str {
        match self {
            AnyFactory::V2(_,) => "v2",
            AnyFactory::V3(_,) => "v3",
        }
    }

    pub fn get_contract(&self,) -> &Contract<Provider<MultiProvider,>,> {
        match self {
            AnyFactory::V2(dex,) | AnyFactory::V3(dex,) => &dex.factory,
        }
    }

    pub fn get_address(&self,) -> H160 {
        match self {
            AnyFactory::V2(dex,) | AnyFactory::V3(dex,) => dex.factory.address(),
        }
    }
}
