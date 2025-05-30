use std::{str::FromStr, string, sync::Arc};

use anyhow::Chain;
use ethers::{
    abi::{Address, Log},
    contract::EthEvent,
    types::{BlockNumber, Filter, ValueOrArray, H160, H256},
    utils::keccak256,
};
use ethers_providers::{Provider, Ws};
use futures::io::Seek;

use crate::{
    arbitro::Arbitro,
    blockchain_db::{DexModel, TokenModel},
    chain_src::ChainSrc,
    chain_svc::ChainDataService,
    factory::AnyFactory,
    mapped_vec::MappedVec,
    mult_provider::MultiProvider,
    pool_action::PoolAction,
    AbisData,
};

pub struct BiggerPicture {
    watchers: Vec<ChainObserver>,
}

pub struct ChainObserver {
    chain_data: ChainData,
    chain_settings: ChainSettings,
    block_service: ChainDataService,
    arbitro: Arbitro,
    chain_src: ChainSrc,
}

impl ChainObserver {
    pub async fn new(chain_data: ChainData, chain_settings: ChainSettings) -> Self {
        let _chain_data = chain_data.clone();
        let arc_ws_provider = Arc::new(_chain_data.ws_providers);
        let arc_http_provider = Arc::new(_chain_data.http_providers);
        let arc_abi_provider = Arc::new(_chain_data.abis);

        let svc = ChainDataService {
            ws_providers: arc_ws_provider,
        };

        let src = ChainSrc::new(
            arc_abi_provider,
            arc_http_provider,
            &chain_settings.tokens,
            &chain_settings.factories, //chain_data.tokens_list,
        )
        .await;

        let abt = Arbitro::new(MappedVec::from_array(src.create_sim().await));

        Self {
            chain_data: chain_data,
            chain_settings: chain_settings,
            block_service: svc,
            arbitro: abt,
            chain_src: src,
        }
    }

    async fn start(&self) {
        // Convert token addresses to H160
        let addresses: Vec<H160> = self
            .chain_settings
            .tokens
            .iter()
            .map(|t| H160::from_str(&t.address))
            .collect::<Result<Vec<_>, _>>()
            .expect("Invalid token address format");

        // Create proper filter
        let filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .address(ValueOrArray::Array(addresses));

        // Spawn the log subscriber
        let mut log_rx = self.block_service.spawn_log_subscriber(filter);

        while let Some(log) = log_rx.recv().await {
            if let Some(action) = PoolAction::parse_pool_action(&log) {
                
            }
        }
    }
}

#[derive(Clone)]
pub struct ChainData {
    id: u32,
    name: String,
    abis: AbisData,
    ws_providers: Vec<Provider<Ws>>,
    http_providers: Provider<MultiProvider>,
}

#[derive(Clone)]
pub struct ChainSettings {
    tokens: Vec<TokenModel>,
    factories: Vec<DexModel>,
}
