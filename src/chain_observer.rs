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
    arbitro::Arbitro, block_decoder::Decoder, blockchain_db::{DexModel, TokenModel}, chain_src::ChainSrc, chain_svc::ChainDataService, factory::AnyFactory, mapped_vec::MappedVec, mem_pool, mult_provider::MultiProvider, pool_action::PoolAction, AbisData
};

pub struct BiggerPicture {
    watchers: Vec<ChainObserver,>,
}

pub struct ChainObserver {
    chain_data: ChainData,
    chain_settings: ChainSettings,
    block_service: ChainDataService,
    arbitro: Arbitro,
    chain_src: ChainSrc,
}

impl ChainObserver {
    pub async fn new(chain_data: ChainData, chain_settings: ChainSettings,) -> Self {
        let _chain_data = chain_data.clone();
        let arc_ws_provider = Arc::new(_chain_data.ws_providers,);
        let arc_http_provider = Arc::new(_chain_data.http_providers,);
        let arc_abi_provider = Arc::new(_chain_data.abis,);

        let svc = ChainDataService {
            ws_providers: arc_ws_provider,
        };

        let src = ChainSrc::new(
            arc_abi_provider,
            arc_http_provider,
            &chain_settings.tokens,
            &chain_settings.factories, // chain_data.tokens_list,
        )
        .await;

        let abt = Arbitro::new(MappedVec::from_array(src.create_sim().await,),);

        Self {
            chain_data,
            chain_settings,
            block_service: svc,
            arbitro: abt,
            chain_src: src,
        }
    }

    async fn start(mut self,) {
        // Convert token addresses to H160
        let addresses: Vec<H160,> = self
            .chain_settings
            .tokens
            .iter()
            .map(|t| H160::from_str(&t.address,),)
            .collect::<Result<Vec<_,>, _,>>()
            .expect("Invalid token address format",);

        // Create proper filter
        let filter = Filter::new().from_block(BlockNumber::Latest,).address(ValueOrArray::Array(addresses,),);

        // Spawn the log subscriber
        let mut log_rx = self.block_service.spawn_log_subscriber(filter,);
        let mut mempool_rx = self.block_service.spawn_mempool_subscriber();
    
        let mut shared_arbitro = Arc::new(Mutex::new(self.arbitro));

        while let Some(mem_pool,) = mempool_rx.recv().await {
            let d = Decoder::decode_tx_static(&self.chain_data.abis,&mem_pool,);
            match d {
                crate::block_decoder::DecodedTx::V2 { func, tokens } => todo!(),
                crate::block_decoder::DecodedTx::V3 { func, tokens } => todo!(),
                crate::block_decoder::DecodedTx::Token { func, tokens } => todo!(),
                crate::block_decoder::DecodedTx::Unknown { selector, to } => todo!(),
            }
        }

        while let Some(log,) = log_rx.recv().await {
            if let Some((action,addr),) = PoolAction::parse_pool_action(&log,) {
                shared_arbitro.lock().await.update_state(&addr, action);
            }
        }
    }

    pub fn process_mempool() {
        
    }
}

#[derive(Clone,)]
pub struct ChainData {
    id: u32,
    name: String,
    abis: AbisData,
    ws_providers: Vec<Provider<Ws,>,>,
    http_providers: Provider<MultiProvider,>,
}

#[derive(Clone,)]
pub struct ChainSettings {
    tokens: Vec<TokenModel,>,
    factories: Vec<DexModel,>,
}
