use bigdecimal::BigDecimal;
use ethers::{
    abi::{token, Tokenizable},
    contract::Contract,
    core::k256::elliptic_curve::consts::{U20, U32},
    providers::{Provider, Ws},
    types::{H160, U256},
    utils::hex::ToHex,
};
use num_traits::{Float, FromPrimitive};
use std::{
    clone,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    fs::read,
    marker::PhantomData,
    ops::{Add, Deref, Div, Mul, Sub},
    path::Path,
    process::id,
    str::FromStr,
    sync::Arc,
    vec,
};

//create tokens and dexes, permutate pools between tokens
//create a graph with the pools
//create paths between tokes without calculating edges
//create map for going back to orinial token without calculating edges
//calculate edges at running time so we can have the most updated values
//calculate the best path


use crate::{
    blockchain_db::{DexModel, TokenModel},
    dex::{self, AnyDex, Dex},
    pair::{self, Pair},
    pool::{Pool, V2Pool},
    pool_utils::{AbisData, AnyPool, PoolDir, Trade},
    token::Token,
};
use bigdecimal::ToPrimitive;
use futures::future;
use tokio::sync::RwLock;

pub struct Arbitro {
    pub dexes: Vec<AnyDex>,
    pub tokens: Vec<Arc<RwLock<Token>>>,
    pub tokens_lookup: HashMap<H160, usize>,
    pub amount: u32,
}

impl Arbitro {
    
    pub fn new(
        dexes_data: &Vec<DexModel>,
        tokens_data: &Vec<TokenModel>,
        provider: Arc<Provider<Ws>>,
        abis: Arc<AbisData>,
    ) -> Self {
        let mut arbitro: Arbitro = Arbitro {
            dexes: vec![],
            tokens: vec![],
            tokens_lookup: HashMap::new(),
            amount: 0,
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
            }else if dexes_data[i].version == "v3" {
                let address_string = &dexes_data[i].factory;
                let address = H160::from_str(address_string).unwrap();
                let contract =
                    Contract::new(address, abis.clone().v3_factory.clone(), provider.clone());
                let dex = Dex {
                    name: dexes_data[i].dex_name.clone(),
                    factory: contract,
                    pools: HashMap::new(),
                };
                let dex_data = AnyDex::V3(dex, abis.clone());
                maybe_dex_data = Some(dex_data);
                
            }

            if let Some(value) = maybe_dex_data {
                println!("DEX criada: {} {}", &value.get_name(), &value.get_version());
                dexes.push(value);
            } else {
                println!("Não foi possível criar a DEX");
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
        let mut addresses = Vec::new();
        let mut _tkns = Vec::new();

        for token_data in tokens_data {
            let addr = match H160::from_str(&token_data.address) {
                Ok(address) => address,
                Err(e) => {
                    eprintln!("Endereço inválido para token {}: {}", token_data.name, e);
                    continue;
                }
            };

            addresses.push(addr);
            let token_contract = Contract::new(addr, abis.bep_20.clone(), provider.clone());

            println!("Token criado: {}", token_data.name);
            _tkns.push(Arc::new(RwLock::new(Token::new(
                token_data.name.clone(),
                addr,
                token_data.symbol.clone(),
                token_data.decimals,
                token_contract,
                HashMap::new(),
            ))));
        }

        self.tokens = _tkns;
        for (idx, addr) in addresses.iter().enumerate() {
            self.tokens_lookup.insert(*addr, idx);
        }
    }

    pub async fn create_pools(&mut self) {
        let mut tokens_addresses = Vec::new();
        for token in &self.tokens {
            let token_guard = token.read().await;
            tokens_addresses.push(token_guard.address);
        }

        if tokens_addresses.len() < 2 {
            return;
        }

        for t0_i in 0..tokens_addresses.len() - 1 {
            for t1_i in t0_i + 1..tokens_addresses.len() {
                let addr_0 = tokens_addresses[t0_i];
                let addr_1 = tokens_addresses[t1_i];

                let t0_idx = match self.tokens_lookup.get(&addr_0) {
                    Some(idx) => *idx,
                    None => continue,
                };
                let t1_idx = match self.tokens_lookup.get(&addr_1) {
                    Some(idx) => *idx,
                    None => continue,
                };

                let token0_clone = {
                    let token = self.tokens[t0_idx].read().await;
                    token.clone()
                };
                let token1_clone = {
                    let token = self.tokens[t1_idx].read().await;
                    token.clone()
                };

                let pair = Pair::new(token0_clone, token1_clone);

                for dex in &mut self.dexes {
                    if let Some(pool) = dex.get_pool(pair.clone()).await {
                        let mut token0 = self.tokens[t0_idx].write().await;
                        let mut token1 = self.tokens[t1_idx].write().await;

                        token0
                            .add_pool(pool.clone(), pair.a.address == addr_0)
                            .await;
                        token1
                            .add_pool(pool.clone(), pair.a.address == addr_1)
                            .await;
                    }
                }
            }
        }
    }

    pub async fn pathfind(&mut self, from: &H160) {
        println!("Iniciando busca de caminhos...");
        self.arbitrage(from, ethers::types::U256::from(10000)).await;
    }

    pub async fn arbitrage(&mut self, from: &H160, amount_in: ethers::types::U256) {
        print!("Iniciando arbitragem...");
        println!("Token de origem: {}", from);
        let mut all_paths: HashMap<H160, Vec<Vec<H160>>> = self.update_paths(from).await;
        println!("Caminhos de retorno atualizados");
        for p in all_paths.iter() {
            println!("Caminhos de retorno de {}: ", p.0);
            for path in p.1 {
                for node in path {
                    print!("{}, ", node);
                }
                println!();
            }
        }
        let connected_nodes = self.get_conected_nodes(from).await;
        println!("Nós conectados: {:?} from {:?}", connected_nodes, from);

        let mut paths: BinaryHeap<(ethers::types::U256, [Vec<Trade>;2])> = BinaryHeap::new();

        for node in connected_nodes {
            if let Some((pool_address, trade_foward)) = self.select_pool(from, &node, U256::from(1000)).await {
                if let Some(return_paths) = all_paths.get(&node) {
                    let (path_backward, best_value) = self
                                                            //amount out is input for the backward trade
                        .select_path(return_paths, trade_foward.amount_out)
                        .await;
                    //print!("Melhor caminho: {:?}", best_path);
                                                        
                    let full_path: [Vec<Trade>; 2] = [vec![trade_foward],path_backward.clone()];

                    paths.push((path_backward.last().unwrap().amount_out, full_path));
                    
                } else {
                    println!("Nenhum caminho de retorno encontrado para o nó {}", node);
                }
            } else {
                println!("Nenhuma pool encontrada para o nó {}", node);
            }
        }



        while let Some((value, path)) = paths.pop() {
            let last_trade = path[1].last().unwrap();
            let last_node = if last_trade.from0 {
                last_trade.token1
            } else {
                last_trade.token0
            };

            if value > amount_in {
                println!(
                    "Arbitragem lucrativa encontrada! Lucro: {}",
                    value - amount_in
                );
            } else {
                println!(
                    "Arbitragem não lucrativa encontrada! Lucro: {}",
                    (value) - (amount_in)
                );
            }
       
            println!("start amount: {} end amount: {}", amount_in, value);
            let start_token_idx = match self.tokens_lookup.get(from) {
                Some(x) => x,
                None => {
                    println!("Token não encontrado");
                    continue;
                }
            };
            let start_token = self.tokens[*start_token_idx].read().await;

            let end_token_idx = match self.tokens_lookup.get(&path[1].last().unwrap().token1) {
                Some(x) => x,
                None => {
                    println!("Token não encontrado");
                    continue;
                }
            };
            let end_token = self.tokens[*end_token_idx].read().await;

            println!("start {} end {}", start_token.symbol, end_token.symbol);
        }
    }

    pub async fn get_conected_nodes(&self, from: &H160) -> Vec<H160> {
        let mut nodes = Vec::new();
        let current_idx = match self.tokens_lookup.get(from) {
            Some(x) => x,
            None => {
                println!("Token não encontrado");
                return nodes;
            }
        };

        let origin_node = self.tokens[*current_idx].read().await;

        for (addr, pool) in &origin_node.pools {
            if let Some(other) = pool.pool.read().await.get_other(from) {
                if !nodes.contains(&other) {
                    nodes.push(other);
                }
            }
        }

        nodes
    }

    pub async fn update_paths(&self, from: &H160) -> HashMap<H160, Vec<Vec<H160>>> {
        let mut paths = HashMap::new();
        let current_idx = match self.tokens_lookup.get(from) {
            Some(idx) => idx,
            None => return paths,
        };

        let origin_node = self.tokens[*current_idx].read().await;
        let mut path_stack = Vec::new();

        for (addr, pool) in &origin_node.pools {
            if let Some(to) = pool.pool.read().await.get_other(from) {
                path_stack.push(vec![from.clone(), to.clone()]);
            }
        }

        let mut add_path = |path: Vec<H160>| {
            if let Some(last) = path.last() {
                paths.entry(*last).or_insert_with(Vec::new).push(path);
            }
        };

        while let Some(current_path) = path_stack.pop() {
            add_path(current_path.clone());

            let last_node = match current_path.last() {
                Some(x) => x,
                None => continue,
            };

            let idx = match self.tokens_lookup.get(last_node) {
                Some(i) => i,
                None => continue,
            };

            let node = self.tokens[*idx].read().await;

            for (addr, pool) in &node.pools {
                if let Some(to) = pool.pool.read().await.get_other(from) {
                    if !current_path.contains(&to) {
                        let mut new_path = current_path.clone();
                        new_path.push(to);
                        path_stack.push(new_path);
                    }
                }
            }
        }

        paths
    }

    pub async fn select_path(
        &mut self,
        paths: &[Vec<H160>],
        amount_in: ethers::types::U256,
    ) -> (Vec<Trade>, U256) {
        let mut best_path = Vec::new();
        let mut best_value = U256::from(0);
        let mut trades = HashMap::new();

        for path in paths {
            let (path_trades, value) = self.calculate_path(path, &mut trades, amount_in).await;
            if value > best_value {
                best_value = value;
                best_path = path_trades;
            }
        }

        (best_path, best_value)
    }

    pub async fn select_pool(
        &self,
        node_address: &H160,
        to: &H160,
        amount_in: U256,
    ) -> Option<(H160, Trade)> {
        println!("Selecionando pool from {} to {}", node_address, to);

        let node_idx = match self.tokens_lookup.get(node_address) {
            Some(x) => x,
            None => {
                println!("Token não encontrado");
                return None;
            }
        };
        let node = self.tokens[*node_idx].read().await;

        let pool_addresses: Vec<H160> = futures::future::join_all(
            node.pools
                .iter()
                .map(|z| async { z.1.pool.read().await.get_address() }),
        )
        .await;
        let pools_with = match node.pools_by_pair.get(to) {
            Some(x) => x.clone(),
            None => {
                println!("Pool não encontrada");
                return None;
            }
        };
        println!("pools with {:?}", &pools_with);
        let mut best_trade = None;

        for pool_addr in pools_with {
            if let Some(pool) = node.pools.get(&pool_addr) {
                let pool_guard = pool.pool.read().await;
                if let Some(trade) = pool_guard.trade(amount_in, pool.is0) {
                    if best_trade
                        .as_ref()
                        .map_or(true, |(pool_addr, t): &(H160, Trade)| {
                            t.amount_out < trade.amount_out
                        })
                    {
                        best_trade = Some((pool_addr, trade));
                    } else {
                    }
                } else {
                    println!("Trade não encontrado");
                }
            } else {
                println!("Pool não encontrada");
            }
        }

        best_trade
    }

    pub async fn calculate_path(
        &mut self,
        path: &[H160],
        trades: &mut HashMap<H160, Trade>,
        amount_in: ethers::types::U256,
    ) -> (Vec<Trade>, U256) {
        let mut current_amount = amount_in;
        let mut trade_sequence = Vec::new();
        
        for i in (1..path.len()).rev() {
            let node_addr = &path[i];
            let next_node_addr = &path[i - 1];

            if let Some(cached_trade) = trades.get(node_addr) {
                current_amount = cached_trade.amount_out;
                trade_sequence.push(cached_trade.clone());
                continue;
            }

            if let Some((_, trade)) = self
                .select_pool(node_addr, next_node_addr, current_amount)
                .await
            {
                current_amount = trade.amount_out;
                trades.insert(*node_addr, trade.clone());
                trade_sequence.push(trade);
            }
        }

        (trade_sequence, current_amount)
    }
}
