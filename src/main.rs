#![allow(warnings)]
mod anvil_test;
mod block_service;
mod block_decoder;
mod blockchain_db;
mod chain_graph;
mod chain_sim;
mod chain_src;
mod err;
mod tick_math;
mod factory;
mod mapped_vec;
mod mem_pool;
mod mult_provider;
mod pair;
mod token;
mod trade;
mod v2_pool_sim;
mod v2_pool_src;
mod v3_pool_sim;
mod v3_pool_src;
mod v_pool_sim;
mod v_pool_src;
use axum::extract::path;
use blockchain_db::{ABIModel, BlockChainsModel, DexModel, TokenModel};
use ethers::abi::{encode, Abi};
use ethers::utils::hex;
use ethers::{contract::*, prelude::*};
use mult_provider::MultiProvider;
use pair::Pair;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env::home_dir;
use std::fs;
use std::process::Command;
use std::time::Duration;
use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};
use tokio::{sync::RwLock, time::error::Error};
use trade::Trade;

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
        "wss://bsc-rpc.publicnode.com".to_string(),
        "wss://bsc-dataseed.binance.org/".to_string(),
        "wss://bsc-dataseed.bnbchain.org".to_string(),
        "wss://bsc-dataseed.nariox.org".to_string(),
        "wss://bsc-dataseed.defibit.io".to_string(),
        "wss://bsc-mainnet.public.blastapi.io".to_string(),
        // If your provider supports wss, otherwise fall back to HTTP for polling:
        "wss://go.getblock.io/84ccc9310a1f40ce9efa177d3e949b3c".to_string(),
    ];
    // Wrap into ethers Provider
    let mut mult_provider: MultiProvider =
        mult_provider::MultiProvider::new(urls, Duration::from_secs(5), 3);
    let abis = load_abis(&chains_data.chains[0].abis);

    let abis_arc = Arc::new(abis);
    let provider = Provider::<MultiProvider>::new(mult_provider);
    let arc_provider = Arc::new(provider);
    let _dexes: &Vec<DexModel> = &chains_data.chains[0].dexes;
    let _tokens: &Vec<TokenModel> = &chains_data.chains[0].tokens;
   

    let mut path = Vec::<Trade>::new();

    let mut chain_src = chain_src::ChainSrc::new(
        abis_arc.clone(),
        arc_provider.clone(),
        ws_urls,
        _tokens,
        _dexes,
    )
    .await;

    Ok(())
}
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
