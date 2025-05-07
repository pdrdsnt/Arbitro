#![allow(warnings)]
mod factory;
mod anvil_test;
mod blockchain_db;
mod mem_pool;
mod mult_provider;
mod pair;
mod err;
mod token;
mod chain_src;
mod chain_sim;
mod v2_pool_src;
mod v2_pool_sim;
mod v3_pool_src;
mod v3_pool_sim;
mod v_pool_sim;
mod v_pool_src;
mod trade;
mod tick;
mod chain_graph;

use axum::extract::path;
use blockchain_db::{ABIModel, BlockChainsModel, DexModel, TokenModel};
use ethers::abi::{encode, Abi};
use ethers::utils::hex;
use ethers::{contract::*, prelude::*};
use mult_provider::MultiProvider;
use pair::Pair;
use serde_json::{json, Value};
use trade::Trade;
use std::collections::HashSet;
use std::env::home_dir;
use std::fs;
use std::process::Command;
use std::time::Duration;
use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};
use tokio::{sync::RwLock, time::error::Error};

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

    // Wrap into ethers Provider
    let mut mult_provider: MultiProvider =
        mult_provider::MultiProvider::new(urls, Duration::from_secs(5), 3);
    let abis = load_abis(&chains_data.chains[0].abis);

    let abis_arc = Arc::new(abis);
    let provider = Provider::<MultiProvider>::new(mult_provider);
    let arc_provider = Arc::new(provider);
    let _dexes = &chains_data.chains[0].dexes;
    let _tokens = &chains_data.chains[0].tokens;
    let tokens = {
        let mut tokens = Vec::new();
        for t in _tokens {
            let add = H160::from_str(&t.address).unwrap();
            let token = token::Token::new(
                t.name.clone(),
                add.clone(),
                t.symbol.clone(),
                t.decimals,
                Contract::new(add,abis_arc.v2_factory.clone(),arc_provider.clone()), 
                //PLACEHOLDER WE DONT CALL TOKEN FUNCTION SO DOESNT MATTER
            );
            tokens.push(Arc::new(RwLock::new(token)));
        }
        tokens
    };
    let mut factories = Vec::new();
    for d in _dexes {
        let add = H160::from_str(&d.address).unwrap();
        let factory = factory::Factory::new(
            
        );
        factories.push(Arc::new(RwLock::new(factory)));
    }
   
    // 1. Get basic stats
    let block: U64 = arc_provider.get_block_number().await.unwrap();
    let v: Value = arc_provider
        .request("txpool_content", Vec::<Value>::new())
        .await
        .unwrap();
    println!("txpool_status: {:#}", block);

    let mut path = Vec::<Trade>::new();
   
    let mut chain_src = chain_src::ChainSrc::new(abis_arc.clone(), arc_provider.clone());
    for y in tokens {chain_src.add_token(y.clone()).await;}
    
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