mod arbitro;
mod blockchain_db;
mod dex;
mod pair;
mod pathfinder;
mod pool;
mod pool_utils;
mod token;

use abi::{Abi, Address};
use arbitro::Arbitro;
use blockchain_db::{BlockChainsModel, DexModel, TokenModel};
use dex::{AnyDex, Dex};
use ethers::{contract::*, prelude::*};
use pair::Pair;
use pool::V2Pool;
use pool_utils::AbisData;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use token::Token;
use tokio::{sync::RwLock, time::error::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {

    let provider = Arc::new(
        Provider::<Ws>::connect("wss://bsc-rpc.publicnode.com")
            .await
            .unwrap(),
    );
    let chains_data = BlockChainsModel::new("src/chainsData.json").unwrap();
    let _dexes = &chains_data.chains[0].dexes;
    let _tokens = &chains_data.chains[0].tokens;

    let abis = &chains_data.chains[0].abis;

    let v2_factory_abi_json = serde_json::to_string(&abis.v2_factory_abi).unwrap();
    let v2_factory_abi_ethers = serde_json::from_str::<Abi>(&v2_factory_abi_json).unwrap();

    let bep_20_abi_json = serde_json::to_string(&abis.bep_20_abi).unwrap();
    let bep_20_abi_ethers = serde_json::from_str::<Abi>(&bep_20_abi_json).unwrap();

    let v3_factory_abi_json = serde_json::to_string(&abis.v3_factory_abi).unwrap();
    let v3_factory_abi_ethers = serde_json::from_str::<Abi>(&v3_factory_abi_json).unwrap();
    
    let v2_pool_abi_json = serde_json::to_string(&abis.v2_pool_abi).unwrap();
    let v2_pool_abi_ethers = serde_json::from_str::<Abi>(&v2_pool_abi_json).unwrap();

    let v3_pool_abi_json = serde_json::to_string(&abis.v3_pool_abi).unwrap();
    let v3_pool_abi_ethers = serde_json::from_str::<Abi>(&v3_pool_abi_json).unwrap();
    
    let abis = AbisData 
    {
        v2_factory: v2_factory_abi_ethers,
        v2_pool: v2_pool_abi_ethers,
        v3_factory: v3_factory_abi_ethers,
        v3_pool: v3_pool_abi_ethers,
        bep_20: bep_20_abi_ethers,
    };

    let abis_arc = Arc::new(abis);
   
    let mut arbitro = Arbitro::new(_dexes,_tokens,provider,abis_arc);
    arbitro.create_pools();
    
    
    Ok(())
}


