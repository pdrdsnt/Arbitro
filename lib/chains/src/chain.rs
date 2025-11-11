use std::{collections::BTreeMap, num::NonZeroUsize, str::FromStr};

use alloy::{
    primitives::{Address, B256},
    providers::{ProviderBuilder, RootProvider},
    rpc::client::RpcClient,
    transports::{
        http::{Http, reqwest::Url},
        layers::FallbackLayer,
    },
};

use bincode::{Decode, Encode};
use dexes::any_pool::AnyPoolKey;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;

use crate::chain_json_model::{ChainDataJsonModel, DexJsonModel, PoolJsonModel, TokenJsonModel};

pub type MyProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        alloy::providers::Identity,
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::GasFiller,
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::BlobGasFiller,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::NonceFiller,
                    alloy::providers::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    RootProvider,
>;

#[derive(Debug)]
pub struct Chain {
    pub id: u64,
    pub tokens: Vec<TokenJsonModel>,
    pub dexes: Vec<DexJsonModel>,
    pub pools: BTreeMap<AnyPoolKey, Vec<PoolJsonModel>>,
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

pub fn generate_fallback_provider(urls: Vec<String>) -> Option<MyProvider> {
    if urls.is_empty() {
        return None;
    }

    let mut trnsports = Vec::new();
    for _url in &urls {
        if let Ok(url) = Url::from_str(_url) {
            let http = Http::new(url);
            trnsports.push(http);
        }
    }

    let count = NonZeroUsize::try_from(trnsports.len()).unwrap();
    let fallback_layer = FallbackLayer::default().with_active_transport_count(count);
    let service = ServiceBuilder::new()
        /*.layer(DBLayer {
            db: chain_db.unwrap(),
        })*/
        .layer(fallback_layer)
        .service(trnsports);

    let client = RpcClient::builder().transport(service, false);

    Some(ProviderBuilder::new().connect_client(client))
}
