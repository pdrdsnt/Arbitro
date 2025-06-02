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

        let svc = ChainDataService { ws_providers: _chain_data.ws_providers.clone() };

        let src = ChainSrc::new(
            _chain_data.abis.clone(),
            _chain_data.http_providers,
            &chain_settings.tokens,
            &chain_settings.factories, // chain_data.tokens_list,
        )
        .await;

        let abt = Arbitro::new(MappedVec::from_array(src.create_sim().await));
        let sim: Simulacrum = Simulacrum::new(abt);
        println!("creating supervisor");
        Self { chain_data, chain_settings, block_service: svc, simulacrum: sim, chain_src: src }
    }
 pub async fn start(mut self) {
        println!("▶ start: Supervisor.entered start()");

        // 1. Convert token addresses to H160
        println!("  • Converting token address strings to H160...");
        let addresses: Vec<H160> = self
            .chain_settings
            .tokens
            .iter()
            .map(|t| {
                println!("    – parsing token {}", t.address);
                H160::from_str(&t.address)
            })
            .collect::<Result<_, _>>()
            .expect("Invalid token address format");
        println!("  ✓ Converted {} token addresses to H160", addresses.len());

        // 2. Create proper filter for logs
        println!("  • Building Filter from_block=Latest, addresses={:?}", addresses);
        let filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .address(ValueOrArray::Array(addresses.clone()));
        println!("  ✓ Log filter created: {:?}", filter);

        // 3. Spawn the log subscriber
        println!("  • Spawning log subscriber...");
        let mut log_rx = self.block_service.spawn_log_subscriber(filter);
        println!("  ✓ Log subscriber spawned");

        // 4. Spawn the mempool subscriber
        println!("  • Spawning mempool subscriber...");
        let mut mempool_rx = self.block_service.spawn_mempool_subscriber();
        println!("  ✓ Mempool subscriber spawned");

        // 5. Wrap simulacrum in an Arc<Mutex<…>> for shared state
        println!("  • Wrapping simulacrum in Arc<Mutex>...");
        let shared_arbitro = Arc::new(Mutex::new(self.simulacrum));
        let shared_arbitro_for_mempool = shared_arbitro.clone();
        println!("  ✓ Simulacrum wrapped (Arc<Mutex>)");

        // 6. Spawn a task to handle mempool events
        println!("  • Spawning tokio task for handling mempool events...");
        tokio::spawn(async move {
            println!("    ▶ mempool task: started");
            while let Some(mem_pool) = mempool_rx.recv().await {
                println!("      • mempool task: received raw tx = {:?}", mem_pool.block_hash);
                let decoded = Decoder::decode_tx_static(&self.chain_data.abis, &mem_pool);
                println!("      • mempool task: decoded tx data = {:?}", decoded);

                if let Some(action) = Decoder::decode_tx_to_action(decoded) {
                    println!("      • mempool task: parsed action = {:?}", action);
                    if let Some(to_addr) = mem_pool.to {
                        println!(
                            "      • mempool task: calling update_phantom_state(to = {:?})",
                            to_addr
                        );
                        shared_arbitro_for_mempool
                            .lock()
                            .await
                            .update_phanton_state(&to_addr, action);
                        println!("      ✓ mempool task: update_phantom_state done");
                    } else {
                        println!("      ! mempool task: tx.to is None, skipping update");
                    }
                } else {
                    println!("      ! mempool task: no PoolAction found, skipping");
                }
            }
            println!("    ◀ mempool task: ended (channel closed)");
        });
        println!("  ✓ Mempool handler task spawned");

        // 7. Enter the main log loop
        println!("  • Entering main loop to process log events…");
        while let Some(log) = log_rx.recv().await {
            println!("    • main loop: received log = {:?}", log);

            if let Some((action, addr)) = PoolAction::parse_pool_action(&log) {
                println!(
                    "    • main loop: parsed PoolAction = {:?}, pool address = {:?}",
                    action, addr
                );
                shared_arbitro
                    .lock()
                    .await
                    .update_main_state(&addr, action);
                println!("    ✓ main loop: update_main_state done");
            } else {
                println!("    ! main loop: log did not match any PoolAction, skipping");
            }
        }
        println!("  ◀ main loop: log channel closed, exiting start()");
    }

}

#[derive(Clone)]
pub struct ChainData {
    pub id: u32,
    pub name: String,
    pub abis: Arc<AbisData>,
    pub ws_providers: Arc<Vec<Provider<Ws>>>,
    pub http_providers: Arc<Provider<MultiProvider>>,
}

#[derive(Clone)]
pub struct ChainSettings {
    pub tokens: Vec<TokenModel>,
    pub factories: Vec<DexModel>,
}
