use bigdecimal::BigDecimal;
use ethers::{
    abi::Tokenizable,
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
    utils::hex::ToHex,
};
use sqlx::pool;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    blockchain_db::{DexModel, TokenModel},
    dex::{self, AnyDex, Dex},
    pair::{self, Pair},
    pathfinder::pathfinder::Pathfinder,
    pool::V2Pool,
    pool_utils::{AbisData, AnyPool, PoolDir},
    token::Token,
};

pub struct Arbitro {
    pub tokens: Vec<Token>,
    pub dexes: Vec<AnyDex>,
    pub pools: Vec<Arc<RwLock<AnyPool>>>,
    pub pathfinder: Pathfinder<H160, Token, i128>,
}

impl Arbitro {
    pub fn new(
        dexes_data: &Vec<DexModel>,
        tokens_data: &Vec<TokenModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,
    ) -> Self {
        let mut arbitro = Arbitro {
            tokens: vec![],
            dexes: vec![],
            pools: vec![],
            pathfinder: Pathfinder {
                space: HashMap::new(),
                edges: HashMap::new(),
                start: H160::zero(),
                target: H160::zero(),
                open: Vec::<H160>::new(),
                closed: Vec::<H160>::new(),
            },
        };
        arbitro.create_dexes(dexes_data, provider.clone(), abis.clone());
        arbitro.create_tokens(tokens_data, provider.clone(), abis.clone());
        println!("Iniciando a criação do Arbitro...");
        println!("Dexes criados: {}", arbitro.dexes.len());
        println!("Tokens criados: {}", arbitro.tokens.len());
        println!("Criação do Arbitro concluída.");

        arbitro
    }

    fn create_dexes(
        &mut self,
        dexes_data: &Vec<DexModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,
    ) {
        let mut dexes = Vec::<AnyDex>::new();
        println!("Iniciando a criação de DEXes...");

        //create dex contracts
        for i in 0..dexes_data.len() {
            let mut maybe_dex_data: Option<AnyDex> = None;
            println!("Processando DEX {}: {}", i + 1, dexes_data[i].dex_name);
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
                maybe_dex_data = Some(dex_data);
            }

            match maybe_dex_data {
                Some(value) => {
                    println!(
                        "DEX just created: {} {}",
                        &value.get_name(),
                        &value.get_version()
                    );
                    dexes.push(value);
                }
                None => {
                    println!("not able to create dex");
                    continue;
                }
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
            let mut pools = HashMap::<H160, PoolDir>::new();

            for p in &*self.pools {
                let cpool = p.blocking_read();
                let (in_pool, is_zero) = (cpool.in_pool(addr), cpool.is_0(addr));
                if !in_pool {
                    continue;
                }
                let pool_with_dir = PoolDir {
                    pool: p.clone(),
                    is0: is_zero,
                };
                pools.entry(cpool.get_address()).or_insert(pool_with_dir);
            }

            println!("token created: {} ", &tokens_data[i].name,);
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

    pub async fn create_pools(&mut self) {
        for t0_i in 0..self.tokens.len() - 1 {
            for t1_i in t0_i + 1..self.tokens.len() {
                let (left, right) = self.tokens.split_at_mut(t1_i);
                let mut token0 = &mut left[t0_i];
                let mut token1 = &mut right[0];
                if token0.address == token1.address {
                    continue;
                }
                print!("token 0 address: {:?} ", token0.address);
                print!("token 1 address: {:?} ", token1.address);

                if let Ok(pair) = Pair::try_from([
                    format!("{:?}", token0.address),
                    format!("{:?}", token1.address),
                ]) {
                    for dex in self.dexes.iter_mut() {
                        if let Some(pool) = dex.get_pool(pair.clone()).await {
                            println!("----------------");
                            println!(
                                "pool created {} ",
                                pool.clone().try_read().unwrap().get_address()
                            );
                            token0.add_pool(pool.clone(), pair.a == token0.address).await;
                            token1.add_pool(pool.clone(), pair.a == token1.address).await;
                            self.pools.push(pool);
                        }
                    }
                }
            }
        }
    }
}
