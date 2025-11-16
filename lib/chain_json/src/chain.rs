use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf, str::FromStr};

use alloy::{
    primitives::{Address, B256},
    providers::{ProviderBuilder, RootProvider},
    rpc::client::RpcClient,
    transports::{
        http::{Http, reqwest::Url},
        layers::FallbackLayer,
    },
};

use crate::chain_json_model::{
    ChainDataJsonModel, DexJsonModel, JsonPoolKey, PoolJsonModel, TokenJsonModel,
};

#[derive(Debug)]
pub struct Chain {
    pub id: u64,
    pub tokens: Vec<TokenJsonModel>,
    pub dexes: Vec<DexJsonModel>,
    pub pools: BTreeMap<JsonPoolKey, Vec<PoolJsonModel>>,
    pub http_nodes_urls: Vec<String>,
}

impl From<ChainDataJsonModel> for Chain {
    fn from(value: ChainDataJsonModel) -> Self {
        Self {
            id: value.id,
            tokens: value.tokens,
            dexes: value.dexes,
            http_nodes_urls: value.http_providers,
            pools: BTreeMap::new(),
        }
    }
}
