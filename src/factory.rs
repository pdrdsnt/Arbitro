use ethers::{
    contract::Contract,
    providers::{Http, Provider},
    types::H160,
};
use std::{collections::HashMap, sync::Arc, vec};
use tokio::sync::RwLock;

use crate::{
    mult_provider::MultiProvider,
    pair::Pair,
    pool_utils::{AbisData, AnyPool},
    token::Token,
    v2_pool::V2Pool,
    v3_pool::V3Pool,
};

#[derive(Clone)]
pub struct Factory {
    pub name: String,
    pub factory: Contract<Provider<MultiProvider>>,
}

#[derive(Clone)]
pub enum AnyFactory {
    V2(Factory, Arc<AbisData>),
    V3(Factory, Arc<AbisData>),
}

impl AnyFactory {
    pub fn supported_fees(&self) -> &'static [u32] {
        match self {
            AnyFactory::V2(_, _) => &[3000], // All V2 DEXes use 0.3% fee
            AnyFactory::V3(dex, _) => match dex.name.as_str() {
                // Major DEXs
                "pancake" => &[500, 2500, 10000], // PancakeSwap V3
                "uniswap" => &[500, 3000, 10000], // Uniswap V3
                "mdex" => &[500, 2000, 10000],    // MDEX V3
                "apeswap" => &[500, 2500, 5000],  // ApeSwap V3
                "biswap" => &[1000, 2000, 3000],  // Biswap V3
                "sushi" => &[500, 3000, 10000],   // SushiSwap V3
                "wault" => &[500, 1500, 3000],    // Wault V3
                "cheeseswap" => &[800, 2000, 5000], // CheeseSwap V3

                // Stablecoin-focused DEXs
                "ellipsis" => &[100, 500, 2000], // Ellipsis V3
                "belt" => &[100, 400, 1500],     // Belt Finance V3

                // Specialized DEXs
                "alpaca" => &[500, 2000, 5000],   // Alpaca Finance V3
                "babyswap" => &[500, 1500, 3000], // BabySwap V3

                _ => &[], // Unknown DEX
            },
        }
    }

    pub fn new(name: String, v2: bool, factory: Contract<Provider<MultiProvider>>, pool_abi: Arc<AbisData>) -> Self {
        let dex = Factory { name, factory };

        if v2 {
            Self::V2(dex, pool_abi)
        } else {
            Self::V3(dex, pool_abi)
        }
    }

    pub async fn get_pool(
        &mut self,
        pair: Pair,
        tkns: &Vec<Arc<RwLock<Token>>>,
        tokens_lookup: &HashMap<H160, usize>,
        fee: u32,
    ) -> Option<Arc<RwLock<AnyPool>>> {
        if !self.supported_fees().contains(&fee) {
            println!("Fee {} não suportada para {}", fee, self.get_name());
            return None;
        }
        let a = pair.a.clone();
        let b = pair.b.clone();

        match self {
            AnyFactory::V2(dex, v2_pool_abi) => {
                let method = dex
                    .factory
                    .method::<(H160, H160), H160>("getPair", (a, b))
                    .unwrap();

                if let Ok(address) = method.call_raw().await {
                    if address == H160::zero() {
                        println!("No pool, returned {}", address);
                        return None;
                    }

                    let pool_contract = Contract::new(
                        address,
                        v2_pool_abi.v2_pool.clone(),
                        dex.factory.client().clone(),
                    );
                    let dex_name = dex.name.clone();

                    let token0_index = tokens_lookup
                        .get(&a)
                        .unwrap_or_else(|| panic!("Token {} not found", a));
                    let token1_index = tokens_lookup
                        .get(&b)
                        .unwrap_or_else(|| panic!("Token {} not found", b));

                    let token0 = tkns[*token0_index].clone();
                    let token1 = tkns[*token1_index].clone();
                    let token0_data = token0.read().await.clone();
                    let token1_data = token1.read().await.clone();

                    let v2_pool = V2Pool::new_with_update(
                        dex_name,
                        "v2".to_string(),
                        3000,
                        address,
                        token0_data,
                        token1_data,
                        pool_contract,
                    )
                    .await;

                    let pool_ref = Arc::new(RwLock::new(AnyPool::V2(v2_pool)));
                    let pool_vec: Vec<Arc<RwLock<AnyPool>>> = vec![pool_ref.clone()];

                    println!("New pool created, returned {}", address);
                    Some(pool_ref)
                } else {
                    None
                }
            }
            AnyFactory::V3(dex, v3_pool_abi) => {
                let method = match dex
                    .factory
                    .method::<(H160, H160, u32), H160>("getPool", (a, b, fee))
                {
                    Ok(m) => m,
                    Err(err) => {
                        println!("No pool: {}", err);
                        return None;
                    }
                };

                if let Ok(address) = method.call_raw().await {
                    if address == H160::zero() {
                        println!("No pool, returned {}", address);
                        return None;
                    } else {
                        println!("Pool address: {}", address);
                    }

                    let pool_contract = Contract::new(
                        address,
                        v3_pool_abi.v3_pool.clone(),
                        dex.factory.client().clone(),
                    );

                    let token0_index = tokens_lookup
                        .get(&a)
                        .unwrap_or_else(|| panic!("Token {} not found", a));
                    let token1_index = tokens_lookup
                        .get(&b)
                        .unwrap_or_else(|| panic!("Token {} not found", b));

                    let token0 = tkns[*token0_index].clone();
                    let token1 = tkns[*token1_index].clone();
                    let token0_data = token0.read().await.clone();
                    let token1_data = token1.read().await.clone();

                    let v3_pool = V3Pool::new_with_update(
                        address,
                        token0_data,
                        token1_data,
                        dex.name.clone(),
                        "v3".to_string(),
                        fee,
                        pool_contract,
                    )
                    .await;

                    let pool_ref = Arc::new(RwLock::new(AnyPool::V3(v3_pool)));
                    let pool_vec = vec![pool_ref.clone()];

                    println!("New pool created, returned {}", address);
                    Some(pool_ref)
                } else {
                    None
                }
            }
        }
    }
   

    pub fn get_name(&self) -> &str {
        match self {
            AnyFactory::V2(dex, _) | AnyFactory::V3(dex, _) => &dex.name,
        }
    }

    pub fn get_version(&self) -> &str {
        match self {
            AnyFactory::V2(_, _) => "v2",
            AnyFactory::V3(_, _) => "v3",
        }
    }

    pub fn get_factory(&self) -> &Contract<Provider<MultiProvider>> {
        match self {
            AnyFactory::V2(dex, _) | AnyFactory::V3(dex, _) => &dex.factory,
        }
    }

    pub fn get_address(&self) -> H160 {
        match self {
            AnyFactory::V2(dex, _) | AnyFactory::V3(dex, _) => dex.factory.address(),
        }
    }
}
