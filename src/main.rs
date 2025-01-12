mod arbitro;
mod blockchain_db;
mod dex;
mod pair;
mod pool;
mod pool_utils;
mod token;
mod pathfinder;

use abi::{Abi, Address};
use blockchain_db::{BlockChains, Dex, Token};
use dex::DexData;
use ethers::{contract::*, prelude::*};
use pair::Pair;
use pool::{Pool, V2Pool};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use token::TokenData;
use tokio::{sync::RwLock, time::error::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut arbitro = arbitro::Arbitro::new();

    let provider = Arc::new(
        Provider::<Ws>::connect("wss://bsc-rpc.publicnode.com")
            .await
            .unwrap(),
    );
    let chains_data = BlockChains::new("src/chainsData.json").unwrap();
    let _dexes = &chains_data.chains[0].dexes;
    let _tokens = &chains_data.chains[0].tokens;
    let tokesn_by_address: HashMap<H160, &Token> = _tokens
        .iter()
        .filter_map(|token| {
            H160::from_str(&token.address)
                .ok()
                .map(|address| (address, token))
        })
        .collect();

    let abis = &chains_data.chains[0].abis;

    let v2_factory_abi_json = serde_json::to_string(&abis.v2_factory_abi).unwrap();
    let v2_factory_abi_ethers = serde_json::from_str::<Abi>(&v2_factory_abi_json).unwrap();
    match serde_json::from_str::<Abi>(&v2_factory_abi_json) {
        Ok(parsed_abi) => {
            // println!("Parsed ABI successfully: {:?}", parsed_abi);
            parsed_abi
        }
        Err(error) => panic!("Failed to parse ABI: {:?}", error),
    };

    let bep_20_abi_json = serde_json::to_string(&abis.bep_20_abi).unwrap();
    let bep_20_abi_ethers = serde_json::from_str::<Abi>(&bep_20_abi_json).unwrap();

    let v3_factory_abi_json = serde_json::to_string(&abis.v3_factory_abi).unwrap();
    let v3_factory_abi_ethers = serde_json::from_str::<Abi>(&v3_factory_abi_json).unwrap();

    let v2_pool_abi_json = serde_json::to_string(&abis.v2_pool_abi).unwrap();
    let v2_pool_abi_ethers = serde_json::from_str::<Abi>(&v2_pool_abi_json).unwrap();

    let v3_pool_abi_json = serde_json::to_string(&abis.v3_pool_abi).unwrap();
    let v3_pool_abi_ethers = serde_json::from_str::<Abi>(&v3_pool_abi_json).unwrap();

    let mut pairs: Vec<Pair> = Vec::new();
    let mut dexes = create_dexes_objects(_dexes, provider.clone(), &v2_factory_abi_ethers);
    let mut v2_pools = Vec::<Arc<V2Pool>>::new();

    //permute tokens to get all unique pairs
    for i in 0.._tokens.len() - 1 {
        for j in i + 1.._tokens.len() {
            let a = _tokens[i].clone();
            let b = _tokens[j].clone();
            let new_pair = Pair::try_from([a.address, b.address]).unwrap();
            pairs.push(new_pair);
        }
    }

    let pairs_arc: Vec<Arc<Pair>> = pairs.into_iter().map(Arc::new).collect();
    let dexes_arc: Vec<Arc<DexData>> = dexes.into_iter().map(Arc::new).collect();

    //create all pools contracts of each pair in each dex
    for pair in &pairs_arc {
        let _token0 = tokesn_by_address[&pair.a];
        let _token1 = tokesn_by_address[&pair.b];

        let _token0_addr = H160::from_str(&_token0.address).unwrap();
        let _token1_addr = H160::from_str(&_token1.address).unwrap();
        println!("a: {}", _token0.name);
        println!("a: {}", _token1.name);
        let token0_contract =
            Contract::new(_token0_addr, bep_20_abi_ethers.clone(), provider.clone());

        let token1_contract =
            Contract::new(_token1_addr, bep_20_abi_ethers.clone(), provider.clone());

        let token0 = TokenData::new(
            _token0.name.clone(),
            _token0_addr,
            _token0.symbol.clone(),
            token0_contract,
        );
        let token1 = TokenData::new(
            _token1.name.clone(),
            _token1_addr,
            _token1.symbol.clone(),
            token1_contract,
        );

        for dex in &dexes_arc {
            if dex.version == "v2" {
                println!("currently on v2 of {}", &dex.name);

                //println!("Calling getPair with tokenA: {:?}, tokenB: {:?}", i.a, i.b);
                let p_clone = Arc::clone(pair);
                let d_clone = Arc::clone(dex);

                let try_raw_call = tokio::spawn(get_pair_address(p_clone, d_clone)).await;

                match try_raw_call {
                    Ok(Ok(address)) => {
                        if address == H160::zero() {
                            println!("no pool, returned {}", address.to_string());

                            println!("=====================");
                            continue;
                        }

                        let v2_pool_contract =
                            Contract::new(address, v2_pool_abi_ethers.clone(), provider.clone());

                        let v2_pool = V2Pool::new_with_update(
                            address,
                            token0.clone(),
                            token1.clone(),
                            v2_pool_contract,
                        )
                        .await;

                        let w_pool = Arc::new(RwLock::new(v2_pool));

                        println!("pair address returned by factory: {}", address.to_string());

                        arbitro.add_v2(_token0.address.clone() , w_pool.clone(), true).await;
                        arbitro.add_v2(_token1.address.clone() , w_pool.clone(), false).await;
                    }

                    Ok(Err(error)) => eprintln!("Low-level call error: {:?}", error),
                    Err(join_error) => eprintln!("Task join error: {:?}", join_error),
                }
            }
        }
    }
    println!("printando arbitro data");
    arbitro.print().await;

    Ok(())
}

async fn get_pair_address(
    pair_data: Arc<Pair>,
    dex_data: Arc<DexData>,
) -> Result<H160, ContractError<Provider<Ws>>> {
    // Attempt to call the "getPair" method on the factory contract
    let method = dex_data
        .factory
        .method::<(Address, Address), Address>("getPair", (pair_data.a, pair_data.b))
        .unwrap();

    // Send the transaction and await the response
    method.call_raw().await
}

fn create_dexes_objects(
    dexes_data: &Vec<Dex>,
    provider: Arc<Provider<Ws>>,
    abi: &Abi,
) -> Vec<DexData> {
    let mut dexes: Vec<DexData> = Vec::<DexData>::new();

    //create dex contracts
    for i in 0..dexes_data.len() {
        let mut maybe_dex_data: Option<DexData> = None;

        if dexes_data[i].version == "v2" {
            let address_string = &dexes_data[i].factory;
            let address = H160::from_str(address_string).unwrap();
            let contract = Contract::new(address, abi.clone(), provider.clone());
            let dex_data: DexData =
                DexData::new(dexes_data[i].dex_name.clone(), "v2".to_string(), contract);
            maybe_dex_data = Some(dex_data);
        }

        match maybe_dex_data {
            Some(value) => dexes.push(value),
            None => continue,
        }
    }

    dexes
}
