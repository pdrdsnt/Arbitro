#![allow(warnings)]
mod anvil_test;
mod arbitro;
mod blockchain_db;
mod chain_graph;
mod chain_src;
mod chain_svc;
mod decoder;
mod err;
mod factory;
mod mapped_vec;
mod ws_manager;
mod mem_pool;
mod mult_provider;
mod pair;
mod pool_action;
mod simulacrum;
mod supervisor;
mod tick_math;
mod token;
mod trade;
mod v2_pool_sim;
mod v2_pool_src;
mod v3_pool_sim;
mod v3_pool_src;
mod v4_pool_src;
mod v_pool_sim;
mod v_pool_src;
use std::{
    collections::{HashMap, HashSet},
    env::home_dir,
    fs,
    path::Path,
    process::Command,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::extract::path;
use blockchain_db::{ABIModel, BlockChainsModel, DexModel, TokenModel};
use ethers::{
    abi::{encode, Abi},
    contract::*,
    prelude::*,
    utils::hex,
};
use futures::future::join_all;
use mult_provider::MultiProvider;
use pair::Pair;
use serde_json::{json, Value};
use tokio::{sync::RwLock, time::error::Error};
use trade::Trade;

use crate::supervisor::{ChainData, ChainSettings, Supervisor};

#[tokio::main]
async fn main() -> Result<(), ethers::providers::ProviderError> {
    let chains_data = BlockChainsModel::new("src/chainsData.json").unwrap();

    let mut urls = vec![
        "https://bsc-rpc.publicnode.com".to_string(),
        "https://bsc-dataseed.binance.org/".to_string(),
        // Additional endpoints
        "https://bsc-dataseed.bnbchain.org".to_string(),
        "https://bsc-dataseed.nariox.org".to_string(),
        "https://bsc-dataseed.defibit.io".to_string(),
        "https://bsc-mainnet.public.blastapi.io".to_string(),
        "https://go.getblock.io/84ccc9310a1f40ce9efa177d3e949b3c".to_string(),
    ];
    let ws_urls: Vec<String> = vec![
        "wss://rpc-bsc.48.club/ws".to_string(),
        "wss://bsc-ws-node.ninicoin.io".to_string(),
        "wss://0xrpc.io/bnb".to_string(),
        "wss://bsc-rpc.publicnode.com".to_string(),
    ]; // 1. Wrap into ethers Provider (HTTP with MultiProvider)
    println!("⏳ 1. Creating MultiProvider…");
    let mut mult_provider: mult_provider::MultiProvider =
        mult_provider::MultiProvider::new(urls.clone(), Duration::from_secs(5), 3);
    println!("✅ 1. MultiProvider created.");

    // 2. Load ABIs
    println!("⏳ 2. Loading ABIs from chains_data…");
    let abis = load_abis(&chains_data.chains[0].abis);
    println!("✅ 2. ABIs loaded ");

    // 3. Wrap ABIs in Arc
    println!("⏳ 3. Wrapping ABIs in Arc…");
    let abis_arc = Arc::new(abis);
    println!("✅ 3. ABIs wrapped in Arc.");

    // 4. Build HTTP provider using our MultiProvider
    println!("⏳ 4. Building HTTP provider from MultiProvider…");
    let http_provider = Provider::<mult_provider::MultiProvider>::new(mult_provider.clone());
    println!("✅ 4. HTTP Provider constructed.");

    // 5. Start connecting all WS providers in parallel
    println!("⏳ 5. Spawning WebSocket connection futures for {} URLs…", ws_urls.len());
    let ws_provider_futures = ws_urls.iter().enumerate().map(|(i, url)| {
        println!("    → [spawn] Connecting to WS URL #{}: {}", i, url);
        Provider::<Ws>::connect(url)
    });
    let ws_results = join_all(ws_provider_futures).await;
    println!("⏳ 5. WS connection futures completed.");

    // 6. Collect and inspect results
    let mut ws_providers: Vec<Provider<Ws>> = Vec::with_capacity(ws_urls.len());
    for (i, result) in ws_results.into_iter().enumerate() {
        match result {
            Ok(provider) => {
                println!("    ✅ [OK] WS #{} connected.", i);
                ws_providers.push(provider);
            }
            Err(err) => {
                println!("    ❌ [ERR] WS #{} failed to connect: {:?}", i, err);
                
            }
        }
    }
    println!("✅ 6. All WS providers connected successfully.");

    // 7. Wrap WS providers in Arc
    println!("⏳ 7. Wrapping {} WS providers in Arc…", ws_providers.len());
    let arc_ws_providers = Arc::new(ws_providers);
    println!("✅ 7. WS providers wrapped in Arc.");

    // 8. Wrap HTTP provider in Arc
    println!("⏳ 8. Wrapping HTTP provider in Arc…");
    let arc_http_provider = Arc::new(http_provider);
    println!("✅ 8. HTTP provider wrapped in Arc.");

    // 9. Clone DEX and token data
    println!("⏳ 9. Cloning dexes and tokens…");
    let dexes: Vec<DexModel> = chains_data.chains[0].dexes.clone();
    let tokens: Vec<TokenModel> = chains_data.chains[0].tokens.clone();
    println!("✅ 9. dexes ({} items) and tokens ({} items) cloned.", dexes.len(), tokens.len());

    // 10. Instantiate Supervisor
    println!("⏳ 10. Creating Supervisor with chain ID {}…", chains_data.chains[0].id);
    let mut supervisor = Supervisor::new(
        ChainData {
            id: chains_data.chains[0].id,
            name: chains_data.chains[0].name.clone(),
            abis: abis_arc.clone(),
            ws_providers: arc_ws_providers.clone(),
            http_providers: arc_http_provider.clone(),
        },
        ChainSettings { tokens: tokens.clone(), factories: dexes.clone() },
    ).await;
    println!("✅ 10. Supervisor created.");

    supervisor.start().await;
    // 12. Everything is up; proceed with main logic
    println!("✅ All setup steps completed successfully. Entering main loop…");

    Ok(())
}
#[derive(Clone)]
// Define the AbisData struct
struct AbisData {
    v2_factory: Abi,
    v2_pool: Abi,
    v3_factory: Abi,
    v3_pool: Abi,
    bep_20: Abi,
}

// Function to load ABIs
fn load_abis(abis: &ABIModel) -> AbisData {
    let v2_factory_abi_json = serde_json::to_string(&abis.v2_factory_abi).unwrap();
    let v2_factory_abi_ethers = serde_json::from_str::<Abi>(&v2_factory_abi_json).unwrap();

    let bep_20_abi_json = serde_json::to_string(&abis.bep_20_abi).unwrap();
    let bep_20_abi_ethers: Abi = serde_json::from_str::<Abi>(&bep_20_abi_json).unwrap();

    let v3_factory_abi_json = serde_json::to_string(&abis.v3_factory_abi).unwrap();
    let v3_factory_abi_ethers = serde_json::from_str::<Abi>(&v3_factory_abi_json).unwrap();

    let v2_pool_abi_json = serde_json::to_string(&abis.v2_pool_abi).unwrap();
    let v2_pool_abi_ethers = serde_json::from_str::<Abi>(&v2_pool_abi_json).unwrap();

    let v3_pool_abi_json = serde_json::to_string(&abis.v3_pool_abi).unwrap();
    let v3_pool_abi_ethers = serde_json::from_str::<Abi>(&v3_pool_abi_json).unwrap();

    AbisData {
        v2_factory: v2_factory_abi_ethers,
        v2_pool: v2_pool_abi_ethers,
        v3_factory: v3_factory_abi_ethers,
        v3_pool: v3_pool_abi_ethers,
        bep_20: bep_20_abi_ethers,
    }
}

// helper:
