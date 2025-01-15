mod arbitro;
mod blockchain_db;
mod dex;
mod pair;
mod pathfinder;
mod pool;
mod pool_utils;
mod token;

use abi::{Abi, Address};
use blockchain_db::{BlockChainsModel, DexModel, TokenModel};
use dex::{AnyDex, Dex};
use ethers::{contract::*, prelude::*};
use pair::Pair;
use pool::V2Pool;
use pool_utils::{AbisData, SomePools};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use token::Token;
use tokio::{sync::RwLock, time::error::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let arbitro = arbitro::Arbitro::new();

    let provider = Arc::new(
        Provider::<Ws>::connect("wss://bsc-rpc.publicnode.com")
            .await
            .unwrap(),
    );
    let chains_data = BlockChainsModel::new("src/chainsData.json").unwrap();
    let _dexes = &chains_data.chains[0].dexes;
    let _tokens = &chains_data.chains[0].tokens;
    let tokesn_by_address: HashMap<H160, &TokenModel> = _tokens
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
        v3_pool: v3_pool_abi_ethers
    };

    let abis_arc = Arc::new(abis);

    let mut pairs: Vec<Pair> = Vec::new();
    let dexes = create_dexes_objects(_dexes, provider.clone(), abis_arc);
    let v2_pools = Vec::<Arc<V2Pool>>::new();

    //permute tokens to get all unique pairs
    for i in 0.._tokens.len() - 1 {
        for j in i + 1.._tokens.len() {
            let a = _tokens[i].clone();
            let b = _tokens[j].clone();
            let new_pair = Pair::try_from([a.address, b.address]).unwrap();
            pairs.push(new_pair);
        }
    }
    
    let tokens: Vec<Arc<RwLock<Token>>> = {
        let mut _tkns = Vec::new();
        for i in 0.._tokens.len() {
            let addr = match H160::from_str(&_tokens[i].address) {
                Ok(address) => address,
                Err(e) => {
                    eprintln!("Invalid address for token {}: {}", _tokens[i].name, e);
                    continue;
                }
            };

            let token_contract = Contract::new(addr, bep_20_abi_ethers.clone(), provider.clone());
            let pools = Arc::new(RwLock::new(SomePools::new(vec![]))); // Replace with actual pool initialization logic

            _tkns.push(Arc::new(RwLock::new(Token::new(
                _tokens[i].name.clone(),
                addr,
                _tokens[i].symbol.clone(),
                token_contract,
                pools,
            ))));
        }
        _tkns
    };
 
    Ok(())
}

async fn get_pair_address(
    pair_data: Arc<Pair>,
    dex_data: Arc<Dex>,
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
    dexes_data: &Vec<DexModel>,
    provider: Arc<Provider<Ws>>,
    abis: Arc<AbisData>,
) -> Vec<AnyDex> {
    let mut dexes: Vec<AnyDex> = Vec::<AnyDex>::new();

    //create dex contracts
    for i in 0..dexes_data.len() {
        let maybe_dex_data: Option<AnyDex> = None;

        if dexes_data[i].version == "v2" {
            let address_string = &dexes_data[i].factory;
            let address = H160::from_str(address_string).unwrap();
            let contract = Contract::new(address, abis.clone().v2_factory.clone(), provider.clone());
            let dex = Dex {name: dexes_data[i].dex_name.clone(),factory: contract,pools: HashMap::new() };
            let dex_data = AnyDex::V2(dex, abis.clone());
        }

        match maybe_dex_data {
            Some(value) => dexes.push(value),
            None => continue,
        }
    }

    dexes
}

