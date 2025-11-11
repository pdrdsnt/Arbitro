use std::{collections::BTreeMap, fs::File, io::Read, ops::Add, str::FromStr};

use alloy::{
    json_abi::Error,
    primitives::{
        Address,
        aliases::{I24, U24},
    },
    providers::Provider,
};
use dexes::{
    any_pool::{AnyPool, AnyPoolKey},
    v2_pool::{V2Data, V2Pool},
    v3_pool::{V3Data, V3Pool},
    v4_pool::{V4Data, V4Pool},
};
use serde::{Deserialize, Serialize};
use sol::sol_types::{
    IUniswapV2Pair::IUniswapV2PairInstance, PoolKey, StateView::StateViewInstance,
    V3Pool::V3PoolInstance,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChainsJsonModel {
    pub chains: Vec<ChainDataJsonModel>,
}

impl BlockChainsJsonModel {
    pub fn new(json_path: &str) -> Result<Self, serde_json::Error> {
        let Ok(mut json_file) = File::open(json_path) else {
            print!("{}", json_path);
            panic!()
        };

        let mut str_json = String::new();
        json_file.read_to_string(&mut str_json).unwrap();

        let blockchains: Result<BlockChainsJsonModel, serde_json::Error> =
            serde_json::from_str(&str_json);

        blockchains
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDataJsonModel {
    pub id: u64,
    pub name: String,
    pub pools: Vec<PoolJsonModel>,
    pub dexes: Vec<DexJsonModel>,
    pub tokens: Vec<TokenJsonModel>,
    pub ws_providers: Vec<String>,
    pub http_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDataJsonModelSmall {
    pub pools: Vec<PoolJsonModel>,
    pub dexes: Vec<DexJsonModel>,
    pub tokens: Vec<TokenJsonModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum DexJsonModel {
    #[serde(rename = "v2")]
    V2 {
        address: String,
        fee: u32,
        stable_fee: Option<u32>,
    },

    #[serde(rename = "v3")]
    V3 { address: String, fee: Vec<u32> },

    #[serde(rename = "v4")]
    V4 { address: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenJsonModel {
    pub name: String,
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
    pub stable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "version")]
pub enum PoolJsonModel {
    #[serde(rename = "v2")]
    V2 {
        factory: Option<String>,
        address: String,
        token0: String,
        token1: String,
        fee: u32,
    },

    #[serde(rename = "v3")]
    V3 {
        factory: Option<String>,
        address: String,
        token0: String,
        token1: String,
        fee: u32,
    },

    #[serde(rename = "v4")]
    V4 {
        factory: Option<String>,
        address: String,
        token0: String,
        token1: String,
        fee: u32,
        spacing: i32,
        hooks: Option<String>,
    },
}

impl PoolJsonModel {
    pub fn to_key(&self) -> Option<AnyPoolKey> {
        match self {
            PoolJsonModel::V2 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let Ok(addr) = Address::from_str(&address) else {
                    return None;
                };
                return Some(AnyPoolKey::V2(addr));
            }
            PoolJsonModel::V3 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let Ok(addr) = Address::from_str(&address) else {
                    return None;
                };
                return Some(AnyPoolKey::V3(addr));
            }
            PoolJsonModel::V4 {
                factory,
                address,
                token0,
                token1,
                fee,
                spacing,
                hooks,
            } => {
                let Ok(t0) = Address::from_str(token0) else {
                    return None;
                };
                let Ok(t1) = Address::from_str(token0) else {
                    return None;
                };
                let Ok(addr) = Address::from_str(token0) else {
                    return None;
                };

                let Ok(sp) = I24::try_from(*spacing) else {
                    return None;
                };

                //-------------------dont need a option here

                let ho = hooks.clone().unwrap_or(Address::ZERO.to_string());

                let Ok(h) = Address::from_str(&ho) else {
                    return None;
                };

                let key = PoolKey {
                    currency0: t0,
                    currency1: t1,
                    fee: U24::from(*fee),
                    tickSpacing: sp,
                    hooks: h,
                };

                return None;
            }
        }
    }

    pub fn to_pool<P: Provider + Clone>(self, provider: P) -> Option<AnyPool<P>> {
        match self {
            PoolJsonModel::V2 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let t0 = Address::from_str(token0.as_str());
                let t1 = Address::from_str(token1.as_str());
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let data = V2Data {
                        name: None,
                        reserves: None,
                        fee: Some(alloy::primitives::aliases::U24::from(fee)),
                        token0: if let Ok(t) = t0 { Some(t) } else { None },
                        token1: if let Ok(t) = t1 { Some(t) } else { None },
                    };
                    let contract = IUniswapV2PairInstance::new(f_addr, provider);
                    Some(AnyPool::V2(V2Pool::new(contract, data)))
                } else {
                    None
                }
            }
            PoolJsonModel::V3 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let t0 = Address::from_str(token0.as_str());
                let t1 = Address::from_str(token1.as_str());
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let data = V3Data {
                        fee: Some(alloy::primitives::aliases::U24::from(fee)),
                        token0: if let Ok(t) = t0 { Some(t) } else { None },
                        token1: if let Ok(t) = t1 { Some(t) } else { None },
                        slot0: None,
                        liquidity: None,
                        ticks: BTreeMap::new(),
                        tick_spacing: None,
                    };
                    let contract = V3PoolInstance::new(f_addr, provider);
                    Some(AnyPool::V3(V3Pool::new(contract, data)))
                } else {
                    None
                }
            }
            PoolJsonModel::V4 {
                factory,
                address,
                token0,
                token1,
                fee,
                spacing,
                hooks,
            } => {
                let Ok(t0) = Address::from_str(token0.as_str()) else {
                    return None;
                };
                let Ok(t1) = Address::from_str(token1.as_str()) else {
                    return None;
                };
                let eth_fee = U24::from(fee);
                let Ok(tick_spacing) = I24::try_from(spacing) else {
                    return None;
                };

                let mut hook = Address::ZERO;
                if let Some(h_str) = hooks {
                    let Ok(h) = Address::from_str(&h_str) else {
                        return None;
                    };
                    hook = h;
                };
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let pool_key = PoolKey {
                        currency0: t0,
                        currency1: t1,
                        fee: eth_fee,
                        tickSpacing: tick_spacing,
                        hooks: hook,
                    };

                    let data = V4Data {
                        slot0: None,
                        liquidity: None,
                        ticks: BTreeMap::new(),
                        pool_key,
                    };

                    let contract = StateViewInstance::new(f_addr, provider);
                    Some(AnyPool::V4(V4Pool::new(contract, data)))
                } else {
                    return None;
                }
            }
        }
    }
}
