use std::{collections::BTreeMap, fs::File, io::Read, ops::Add, str::FromStr};

use alloy::{
    json_abi::Error,
    primitives::{
        Address, B256,
        aliases::{I24, U24},
    },
    providers::Provider,
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
    pub fn to_key(&self) -> Option<JsonPoolKey> {
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
                return Some(JsonPoolKey::V2(addr));
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
                return Some(JsonPoolKey::V3(addr));
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
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JsonPoolKey {
    V2(Address),
    V3(Address),
    V4(Address, B256),
}
