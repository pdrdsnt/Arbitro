use std::{str::FromStr, string, sync::Arc};

use anyhow::Chain;
use ethers::{
    abi::{Address, Log},
    contract::EthEvent,
    types::{BlockNumber, Filter, Transaction, ValueOrArray, H160, H256},
    utils::keccak256,
};
use ethers_providers::{Provider, Ws};
use futures::io::Seek;
use tokio::sync::Mutex;

use crate::{
    arbitro::Arbitro,
    decoder::Decoder,
    blockchain_db::{DexModel, TokenModel},
    chain_src::ChainSrc,
    chain_svc::ChainDataService,
    factory::AnyFactory,
    mapped_vec::MappedVec,
    mem_pool,
    mult_provider::MultiProvider,
    pool_action::PoolAction,
    simulacrum::Simulacrum,
    v_pool_sim::AnyPoolSim,
    AbisData,
};

pub struct Supervisor {
    chain_data: ChainData,
    chain_settings: ChainSettings,
    block_service: ChainDataService,
    simulacrum: Simulacrum,
    chain_src: ChainSrc,
}

impl Supervisor {
    pub async fn new(chain_data: ChainData, chain_settings: ChainSettings) -> Self {
        let _chain_data = chain_data.clone();
        let arc_ws_provider = Arc::new(_chain_data.ws_providers);
        let arc_http_provider = Arc::new(_chain_data.http_providers);
        let arc_abi_provider = Arc::new(_chain_data.abis);

        let svc = ChainDataService { ws_providers: arc_ws_provider };

        let src = ChainSrc::new(
            arc_abi_provider,
            arc_http_provider,
            &chain_settings.tokens,
            &chain_settings.factories, // chain_data.tokens_list,
        )
        .await;

        let abt = Arbitro::new(MappedVec::from_array(src.create_sim().await));
        let sim: Simulacrum = Simulacrum::new(abt);
        Self { chain_data, chain_settings, block_service: svc, simulacrum: sim, chain_src: src }
    }

    async fn start(mut self) {
        // Convert token addresses to H160
        let addresses: Vec<H160> = self
            .chain_settings
            .tokens
            .iter()
            .map(|t| H160::from_str(&t.address))
            .collect::<Result<Vec<_>, _>>()
            .expect("Invalid token address format");

        // Create proper filter
        let filter =
            Filter::new().from_block(BlockNumber::Latest).address(ValueOrArray::Array(addresses));

        // Spawn the log subscriber
        let mut log_rx = self.block_service.spawn_log_subscriber(filter);
        let mut mempool_rx = self.block_service.spawn_mempool_subscriber();

        let shared_arbitro = Arc::new(Mutex::new(self.simulacrum));
        let _shared_arbitro = shared_arbitro.clone();
        tokio::spawn(async move {
            while let Some(mem_pool) = mempool_rx.recv().await {
                let d = Decoder::decode_tx_static(&self.chain_data.abis, &mem_pool);
                let r = Decoder::decode_tx_to_action(d);
            }
        });

        while let Some(log) = log_rx.recv().await {
            if let Some((action, addr)) = PoolAction::parse_pool_action(&log) {
                shared_arbitro.lock().await.origin_mut().update_state(&addr, action);
            }
        }
    }

    pub fn process_mempool() {
        // duplicate arbitro and update state of one pool in the new arbitro
        // maybe a new struc that manages multiple arbitros and the specific modifications
        // mempool operations can fail so multiple possibilities
        // pools - modifications -
        // H160  - [{block, provider, trade}]
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
