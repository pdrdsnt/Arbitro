use std::{collections::HashMap, sync::Arc, vec};

use ethers::{
    contract::Contract,
    core::k256::elliptic_curve::consts::U24,
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
    pub pools: HashMap<Pair, Vec<Arc<RwLock<AnyPool>>>>,
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

    pub async fn get_pool(&mut self, pair: Pair, fee: u32) -> Option<Arc<RwLock<AnyPool>>> {
        let a = pair.a.clone();
        let b = pair.b.clone();
        match self {
            AnyDex::V2(dex, v2_pool_abi_ethers) => {
                if fee != 3000 {
                    println!("fee not supported");
                    return None;
                }
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

                    let dex_name = dex.name.clone();

                    //v2 pool only one fee
                    //fee should depend on dex data because some dex have fees other than 3000
                    let v2_pool = V2Pool::new_with_update(
                        dex_name,
                        "v2".to_string(),
                        3000,
                        address,
                        a,
                        b,
                        v2_pool_contract,
                    )
                    .await;
                    let p = Arc::new(RwLock::new(AnyPool::V2(v2_pool)));
                    let anypool = vec![p.clone()];

                    dex.pools
                        .entry(pair.clone())
                        .and_modify(|v| *v = anypool.clone());
                    if let Some(pool) = dex.pools.get_mut(&pair) {
                    } else {
                        dex.pools.insert(pair.clone(), anypool.clone());
                    }
                    println!("new pool, returned {}", address);
                    Some(p.clone())
                } else {
                    None
                }
            }
            AnyDex::V3(dex, v3_pool_abi_ethers) => {
                let method = match dex
                    .factory
                    .method::<(H160, H160, u32), H160>("getPool", (a.address, b.address, fee))
                {
                    Ok(v) => v,
                    Err(err) => {
                        println!("no pool, returned None");
                        println!("{}", err);
                        return None;
                    }
                };

                if let Ok(address) = method.call_raw().await {
                    let v3_pool_contract = Contract::new(
                        address,
                        v3_pool_abi_ethers.v3_pool.clone(),
                        dex.factory.client().clone(),
                    );

                    if address == H160::zero() {
                        println!("no pool, returned {}", address);
                        return None;
                    }
                    else {
                        println!("pool address {}", address);
                    }
                    let v3_pool = V3Pool::new_with_update(
                        address,
                        a,
                        b,
                    dex.name.to_string(),
                        "v3".to_string(),
                        fee,
                        v3_pool_contract
                    )
                    .await;
                    let p = Arc::new(RwLock::new(AnyPool::V3(v3_pool)));
                    let anypool = vec![p.clone()];

                    dex.pools
                        .entry(pair.clone())
                        .and_modify(|v| *v = anypool.clone());
                    if let Some(pool) = dex.pools.get_mut(&pair) {
                    } else {
                        dex.pools.insert(pair.clone(), anypool.clone());
                    }
                    println!("new pool, returned {}", address);
                    Some(p.clone())
                } else {
                    None
                }
            }
        }
    }

    pub fn add_pool(&mut self, pair: Pair, pool: Arc<RwLock<AnyPool>>) {
        match self {
            AnyDex::V2(dex, _) | AnyDex::V3(dex, _) => {
                dex.pools
                    .entry(pair)
                    .and_modify(|x| x.push(pool.clone()))
                    .or_insert(vec![pool.clone()]);
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
