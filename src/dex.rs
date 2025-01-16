use std::{collections::HashMap, sync::Arc};

use ethers::{
    contract::Contract, providers::{Provider, Ws}, types::H160
};
use tokio::sync::RwLock;

use crate::{
    pair::Pair,
    pool::V2Pool,
    pool_utils::{AbisData, AnyPool},
};

pub struct Dex {
    pub name: String,
    pub factory: Contract<Provider<Ws>>,
    pub pools: HashMap<Pair, Arc<RwLock<AnyPool>>>,
}

pub enum AnyDex {
    V2(Dex, Arc<AbisData>),
    V3(Dex, Arc<AbisData>),
}

impl AnyDex {
    pub fn new(name: String, v2: bool, factory: Contract<Provider<Ws>>,pool_abi: Arc<AbisData>) -> Self {
        if v2 {
            Self::V2(Dex {
                name,
                factory,
                pools: HashMap::new(),
            },pool_abi)
        } else {
            Self::V3(Dex {
                name,
                factory,
                pools: HashMap::new(),
            },pool_abi)
        }
    }

    pub async fn get_pool(&mut self, pair: Pair) -> Arc<RwLock<AnyPool>> {
        let a = pair.a;
        let b = pair.b;
        match self {
            AnyDex::V2(dex,v2_pool_abi_ethers) => {
                let method = dex
                    .factory
                    .method::<(H160, H160), H160>("getPair", (a, b))
                    .unwrap();

                // Send the transaction and await the response
                let address = method.call_raw().await.unwrap();

                if address == H160::zero() {
                    println!("no pool, returned {}", address);

                    println!("=====================");
                    todo!()
                }

                let v2_pool_contract = Contract::new(
                    address,
                    v2_pool_abi_ethers.v2_pool.clone(),
                    dex.factory.client().clone(),
                );

                let v2_pool = V2Pool::new_with_update(address, a, b, v2_pool_contract).await;

                

                let anypool = Arc::new(RwLock::new(AnyPool::V2(v2_pool)));

                dex.pools.entry(pair).and_modify(|v| *v = anypool.clone());

                anypool.clone()
            }
            AnyDex::V3(dex,v3_pool_abi_ethers) => todo!(),
        }
    }

    pub fn add_pool(&mut self, pair: Pair, pool: Arc<RwLock<AnyPool>>) {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => {
                dex.pools.insert(pair, pool);
            }
        }
    }

    pub fn get_pools_with_token(&self, token: H160) -> Vec<(Pair, Arc<RwLock<AnyPool>>, bool)> {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => {
                let mut pools = Vec::<(Pair,Arc<RwLock<AnyPool>>, bool)>::new();
                for (key, pool) in dex.pools.iter() {
                    if let Ok(pair) = Pair::try_from(key.clone()) {
                        if pair.a == token {
                            pools.push((key.clone(), pool.clone(), true)); // `true` indica que o token é `a`
                        } else if pair.b == token {
                            pools.push((key.clone(), pool.clone(), false)); // `false` indica que o token é `b`
                        }
                    }
                }
                pools
            }
        }
    }
}
