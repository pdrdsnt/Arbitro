use bigdecimal::BigDecimal;
use ethers::{
    abi::Tokenizable,
    contract::Contract,
    providers::{Provider, Ws},
    types::H160,
    utils::hex::ToHex,
};
use std::{
    clone,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    str::FromStr,
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{
    blockchain_db::{DexModel, TokenModel},
    dex::{self, AnyDex, Dex},
    pair::{self, Pair},
    pathfinder::pathfinder::Pathfinder,
    pool::V2Pool,
    pool_utils::{AbisData, AnyPool, PoolDir, Trade},
    token::Token,
};

pub struct Arbitro {
    pub dexes: Vec<AnyDex>,

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
            dexes: vec![],
            pathfinder: Pathfinder {
                space: HashMap::new(),
                start: H160::zero(),
                target: H160::zero(),
                open: HashSet::<Trade>::new(),
                closed: HashSet::<Trade>::new(),
                h: PhantomData,
            },
        };
        arbitro.create_dexes(dexes_data, provider.clone(), abis.clone());
        arbitro.create_tokens(tokens_data, provider.clone(), abis.clone());
        println!("Iniciando a criação do Arbitro...");
        println!("Dexes criados: {}", arbitro.dexes.len());
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

            let decimals = tokens_data[i].decimals;

            println!("token created: {} ", &tokens_data[i].name,);
            _tkns.push(Token::new(
                tokens_data[i].name.clone(),
                addr,
                tokens_data[i].symbol.clone(),
                decimals,
                token_contract,
                pools,
            ));
        }
    }

    pub async fn create_pools(&mut self) {
        let mut tokens_addresses: Vec<H160> = self.pathfinder.space.keys().cloned().collect();
        if tokens_addresses.len() < 2 {return;}
        for t0_i in 0..tokens_addresses.len() - 1 {
            for t1_i in t0_i + 1..tokens_addresses.len() {
                let addr_0 = &tokens_addresses[t0_i];
                let addr_1 = &tokens_addresses[t1_i];

                let mut _token0 = self.pathfinder.space.get(addr_0).unwrap();
                let mut _token1 = self.pathfinder.space.get(addr_1).unwrap();

                let mut token0 = _token0.write().await;
                let mut token1 = _token1.write().await;

                if token0.address == token1.address {
                    continue;
                }
                print!("token 0 address: {:?} ", token0.address);
                print!("token 1 address: {:?} ", token1.address);

                let pair = Pair::new(token0.clone(), token1.clone());
                for dex in self.dexes.iter_mut() {
                    if let Some(pool) = dex.get_pool(pair.clone()).await {
                        println!("----------------");
                        println!(
                            "pool created {} ",
                            pool.clone().try_read().unwrap().get_address()
                        );
                        token0
                            .add_pool(pool.clone(), pair.a.address == addr_0.clone())
                            .await;
                        token1
                            .add_pool(pool.clone(), pair.a.address == addr_1.clone())
                            .await;
                    }
                }
            }
        }
    }

    pub async fn pathfind(&mut self, start: H160) {}
}
