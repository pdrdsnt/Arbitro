use std::{collections::HashMap, sync::Arc};

use ethers::{
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
};
use tokio::sync::RwLock;

use crate::{
    pair::Pair,
    pool::{V2Pool, V3Pool},
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
    pub fn new(
        name: String,
        v2: bool,
        factory: Contract<Provider<Ws>>,
        pool_abi: Arc<AbisData>,
    ) -> Self {
        if v2 {
            Self::V2(
                Dex {
                    name,
                    factory,
                    pools: HashMap::new(),
                },
                pool_abi,
            )
        } else {
            Self::V3(
                Dex {
                    name,
                    factory,
                    pools: HashMap::new(),
                },
                pool_abi,
            )
        }
    }

    pub async fn get_pool(&mut self, pair: Pair) -> Option<Arc<RwLock<AnyPool>>> {
        let a = pair.a.clone();
        let b = pair.b.clone();
        match self {
            AnyDex::V2(dex, v2_pool_abi_ethers) => {
                let method = dex
                    .factory
                    .method::<(H160, H160), H160>("getPair", (a.address, b.address))
                    .unwrap();

                // Send the transaction and await the response
                if let Ok(address) = method.call_raw().await {
                    if address == H160::zero() {
                        println!("no pool, returned {}", address);

                        println!("=====================");
                        return None;
                    }

                    let v2_pool_contract = Contract::new(
                        address,
                        v2_pool_abi_ethers.v2_pool.clone(),
                        dex.factory.client().clone(),
                    );

                    let v2_pool = V2Pool::new_with_update(address, a, b, v2_pool_contract).await;

                    let anypool = Arc::new(RwLock::new(AnyPool::V2(v2_pool)));

                    dex.pools
                        .entry(pair.clone())
                        .and_modify(|v| *v = anypool.clone());
                    if let Some(pool) = dex.pools.get_mut(&pair) {
                    } else {
                        dex.pools.insert(pair.clone(), anypool.clone());
                    }
                    println!("new pool, returned {}", address);
                    Some(anypool.clone())
                } else {
                    None
                }
            }
            AnyDex::V3(dex, v3_pool_abi_ethers) => {
                let method = dex
                    .factory
                    .method::<(H160, H160), H160>("getPool", (a.address, b.address))
                    .unwrap();

                if let Ok(address) = method.call_raw().await {
                    let v3_pool_contract = Contract::new(
                        address,
                        v3_pool_abi_ethers.v3_pool.clone(),
                        dex.factory.client().clone(),
                    );

                    let v3_pool = V3Pool::new_with_update(address, a, b, v3_pool_contract).await;
                    let anypool = Arc::new(RwLock::new(AnyPool::V3(v3_pool)));

                    dex.pools
                        .entry(pair.clone())
                        .and_modify(|v| *v = anypool.clone());
                    if let Some(pool) = dex.pools.get_mut(&pair) {
                    } else {
                        dex.pools.insert(pair.clone(), anypool.clone());
                    }
                    println!("new pool, returned {}", address);
                    Some(anypool.clone())
                }else {
                    None
                }
            }
        }
    }

    pub fn add_pool(&mut self, pair: Pair, pool: Arc<RwLock<AnyPool>>) {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => {
                dex.pools.insert(pair, pool);
            }
            AnyDex::V3(dex, _) | AnyDex::V3(dex, _) => {
                dex.pools.insert(pair, pool);
            }
        }
    }

    pub fn get_pools_with_token(&self, token: H160) -> Vec<(Pair, Arc<RwLock<AnyPool>>, bool)> {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => {
                let mut pools = Vec::<(Pair, Arc<RwLock<AnyPool>>, bool)>::new();
                for (key, pool) in dex.pools.iter() {
                    if let Ok(pair) = Pair::try_from(key.clone()) {
                        if pair.a.address == token {
                            pools.push((key.clone(), pool.clone(), true)); // `true` indica que o token é `a`
                        } else if pair.b.address == token {
                            pools.push((key.clone(), pool.clone(), false)); // `false` indica que o token é `b`
                        }
                    }
                }
                pools
            }
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => &dex.name,
        }
    }

    pub fn get_version(&self) -> &str {
        match self {
            AnyDex::V2(_, _) => "v2",
            AnyDex::V3(_, _) => "v3",
        }
    }
}
