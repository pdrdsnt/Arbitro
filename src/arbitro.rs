use ethers::{
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
};
use sqlx::pool;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    blockchain_db::{DexModel, TokenModel}, dex::{self, AnyDex, Dex}, pair::Pair, pathfinder::pathfinder::Pathfinder, pool::V2Pool, pool_utils::{AbisData, AnyPool, PoolDir, SomePools}, token::Token
};

pub struct Arbitro {
    pub tokens: Vec<Token>,
    pub dexes: Vec<AnyDex>,
    pub pools: Vec<Arc<RwLock<AnyPool>>>,
}

impl Arbitro {

    pub fn new (
        dexes_data: &Vec<DexModel>,
        tokens_data: &Vec<TokenModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,) -> Self
        {
            let mut arbitro = Arbitro { tokens: vec![] , dexes: vec![] , pools: vec![]};
            arbitro.create_dexes(dexes_data, provider.clone(), abis.clone());
            arbitro.create_tokens(tokens_data, provider.clone(), abis.clone());
            arbitro
        } 

    fn create_dexes(
        &mut self,
        dexes_data: &Vec<DexModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,
    ) {
        let mut dexes = Vec::<AnyDex>::new();

        //create dex contracts
        for i in 0..dexes_data.len() {
            let maybe_dex_data: Option<AnyDex> = None;

            if dexes_data[i].version == "v2" {
                let address_string = &dexes_data[i].factory;
                let address = H160::from_str(address_string).unwrap();
                let contract =
                    Contract::new(address, abis.clone().v2_factory.clone(), provider.clone());
                let dex = Dex {
                    name: dexes_data[i].dex_name.clone(),
                    factory: contract,
                    pools: HashMap::new(),
                };
                let dex_data = AnyDex::V2(dex, abis.clone());
            }

            match maybe_dex_data {
                Some(value) => dexes.push(value),
                None => continue,
            }
        }

        self.dexes = dexes;
    }

    fn create_tokens(
        &mut self,
        tokens_data: &Vec<TokenModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,
    ) {
        let mut _tkns = Vec::new();
        for i in 0..tokens_data.len() {
            let addr = match H160::from_str(&tokens_data[i].address) {
                Ok(address) => address,
                Err(e) => {
                    eprintln!("Invalid address for token {}: {}", tokens_data[i].name, e);
                    continue;
                }
            };

            let token_contract = Contract::new(addr, abis.bep_20.clone(), provider.clone());
            let pools = SomePools::new(vec![]); // Replace with actual pool initialization logic

            _tkns.push(Token::new(
                tokens_data[i].name.clone(),
                addr,
                tokens_data[i].symbol.clone(),
                token_contract,
                pools,
            ));
        }
        
        self.tokens = _tkns;
        
    }

    pub async fn create_pools(&mut self){
        for t0_i in 0..self.tokens.len() - 1{
            for t1_i in t0_i + 1..self.tokens.len(){
                let (left, right) = self.tokens.split_at_mut(t1_i);
                let mut token0 = &mut left[t0_i];
                let mut token1 = &mut right[0];
                if token0.address == token1.address {continue;}
                let pair = Pair::try_from([token0.address.to_string(),token1.address.to_string()]).unwrap();
                
                for dex in self.dexes.iter_mut() {
                    let pool = dex.get_pool(pair.clone()).await;
                    token0.add_pool(pool.clone(), pair.a == token0.address);
                    token1.add_pool(pool.clone(), pair.a == token1.address);
                    self.pools.push(pool);
                }
            }
        }

    }
}
