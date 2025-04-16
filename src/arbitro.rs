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
use sqlx::any;
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
            } else if dexes_data[i].version == "v3" {
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

                let pair = Pair::new(token0_clone.address, token1_clone.address);

                let fees = vec![1000, 3000, 500, 10000];

                for dex in &mut self.dexes {
                    for fee in &fees {
                        if let Some(pool) = dex
                            .get_pool(pair.clone(), &self.tokens, &self.tokens_lookup, fee.clone())
                            .await
                        {
                            let mut token0 = self.tokens[t0_idx].write().await;
                            let mut token1 = self.tokens[t1_idx].write().await;

                            token0.add_pool(pool.clone(), pair.a == addr_0).await;
                            token1.add_pool(pool.clone(), pair.a == addr_1).await;
                        }
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
        print!("=========================");
        println!("==========================");

        let connected_nodes = self.find_return_paths(from,4).await;
        let mut all_paths: HashMap<H160, Vec<Vec<H160>>> = self.update_paths(from).await;
        println!("Caminhos de retorno atualizados");

        let mut paths: BinaryHeap<(ethers::types::U256, [Vec<Trade>; 2])> = BinaryHeap::new();
        for (node,path) in connected_nodes {
            println!("connected {} ---> {:?}", from, node);
            if let Some((pool_address, trade_foward)) =
                self.select_pool(from, &node, amount_in).await
            {
                println!(
                    "foward trade encontrado: {} {} -> {}: {} from0: {} on pool {}",
                    trade_foward.amount_in,
                    trade_foward.token0,
                    trade_foward.token1,
                    trade_foward.amount_out,
                    trade_foward.from0,
                    trade_foward.pool
                );
                if let Some(return_paths) = all_paths.get(&node) {
                    
                    let (mut path_backward, best_value) = self
                        //amount out is input for the backward trade
                        .select_path(&return_paths, trade_foward.amount_out)
                        .await;

                    println!("Caminho de retorno encontrado para no {:?}", node);

                    for t in path_backward.iter_mut() {
                        println!(
                            "backward trade encontrado: {} {} -> {}: {} from0: {} on pool {}",
                            t.amount_in, t.token0, t.token1, t.amount_out, t.from0, t.pool
                        );
                    }
                    let full_path: [Vec<Trade>; 2] = [vec![trade_foward], path_backward.clone()];
                    let last_trade = match path_backward.last() {
                        Some(v) => v.amount_out,
                        None => {
                            println!("last trade não encontrado");
                            continue;
                        }
                    };
                    paths.push((last_trade, full_path));

                    println!(
                        "Valor total: {}",
                        if last_trade > amount_in {
                            last_trade - amount_in
                        } else {
                            amount_in - last_trade
                        }
                    );
                } else {
                    println!("Nenhum caminho de retorno encontrado para o nó {}", node);
                }
            } else {
                println!("Nenhuma pool encontrada para o nó {}", node);
            }
        }
        println!("inicializacao conclioda");
        println!("=========================");
        println!("==========================");
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
                    match value.checked_sub(amount_in) {
                        Some(v) => v,
                        None => {
                            println!("Valor negativo");
                            continue;
                        }
                    }
                );
            } else {
                let value = amount_in - value;
                println!("Valor negativo -{}", value);
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

            println!("path foward");

            for t in &path[0] {
                println!(
                    "DEBUG: from0 = {}, token0 = {:?}, token1 = {:?}",
                    t.from0, t.token0, t.token1
                );

                let end_token0_idx = match self.tokens_lookup.get(&t.token0) {
                    Some(x) => x,
                    None => {
                        println!("Token não encontrado");
                        continue;
                    }
                };
                let end_token0 = self.tokens[*end_token0_idx].read().await;

                let end_token1_idx = match self.tokens_lookup.get(&t.token1) {
                    Some(x) => x,
                    None => {
                        println!("Token não encontrado");
                        continue;
                    }
                };
                let end_token1: tokio::sync::RwLockReadGuard<'_, Token> =
                    self.tokens[*end_token1_idx].read().await;

                println!(
                    "{}: {} -> {}: {} -- on pool: {} {} {}",
                    if t.from0 {
                        &end_token0.symbol
                    } else {
                        &end_token1.symbol
                    },
                    t.amount_in,
                    if t.from0 {
                        &end_token1.symbol
                    } else {
                        &end_token0.symbol
                    },
                    t.amount_out,
                    t.dex,
                    t.version,
                    t.fee
                );
            }

            println!("path back");

            for t in &path[1] {
                println!(
                    "DEBUG: from0 = {}, token0 = {:?}, token1 = {:?}",
                    t.from0, t.token0, t.token1
                );

                let end_token0_idx = match self.tokens_lookup.get(&t.token0) {
                    Some(x) => x,
                    None => {
                        println!("Token não encontrado");
                        continue;
                    }
                };
                let end_token0 = self.tokens[*end_token0_idx].read().await;

                let end_token1_idx = match self.tokens_lookup.get(&t.token1) {
                    Some(x) => x,
                    None => {
                        println!("Token não encontrado");
                        continue;
                    }
                };
                let end_token1 = self.tokens[*end_token1_idx].read().await;

                println!(
                    "{}: {} -> {}: {} -- on pool: {} v: {}",
                    if t.from0 {
                        &end_token0.symbol
                    } else {
                        &end_token1.symbol
                    },
                    t.amount_in,
                    if t.from0 {
                        &end_token1.symbol
                    } else {
                        &end_token0.symbol
                    },
                    t.amount_out,
                    t.dex,
                    t.version
                );
            }
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

    pub async fn update_paths(&self, start: &H160) -> HashMap<H160, Vec<Vec<H160>>> {
        let mut paths = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back((vec![*start], 0)); // Rastreia profundidade

        while let Some((current_path, depth)) = queue.pop_front() {
            let last_node = current_path.last().unwrap();

            // Limita a profundidade máxima (ex: 4 hops)
            if depth >= 4 {
                continue;
            }

            let idx = match self.tokens_lookup.get(last_node) {
                Some(i) => *i,
                None => continue,
            };
            let node = self.tokens[idx].read().await;
            let connected_nodes = self.get_conected_nodes(last_node).await;

            for (addr) in connected_nodes {
                if addr != *start && current_path.contains(&addr) {
                    continue;
                }
                let mut new_path = current_path.clone();
                new_path.push(addr);
                paths
                    .entry(addr)
                    .or_insert_with(Vec::new)
                    .push(new_path.clone());
                queue.push_back((new_path, depth + 1));
            }
        }
        paths
    }
    pub async fn find_return_paths(
        &self,
        start: &H160,
        max_depth: usize,
    ) -> HashMap<H160, Vec<Vec<H160>>> {
        let mut return_paths = HashMap::new();
        let connected_nodes = self.get_conected_nodes(start).await;
    
        // Para cada nó conectado ao token inicial
        for node in &connected_nodes {
            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            
            queue.push_back((vec![*node], 0));  // Começa do nó conectado
            
            while let Some((current_path, depth)) = queue.pop_front() {
                if depth > max_depth {
                    continue;
                }
    
                let last_node = current_path.last().unwrap();
                
                // Encontrou caminho de volta para o token inicial
                if last_node == start {
                    return_paths.entry(*node)
                        .or_insert_with(Vec::new)
                        .push(current_path);
                    continue;
                }
    
                if visited.contains(last_node) {
                    continue;
                }
                visited.insert(*last_node);
    
                let connected = self.get_conected_nodes(last_node).await;
                for next_node in connected {
                    // Evita ciclos exceto para retorno ao start
                    if next_node != *start && current_path.contains(&next_node) {
                        continue;
                    }
                    
                    let mut new_path = current_path.clone();
                    new_path.push(next_node);
                    queue.push_back((new_path, depth + 1));
                }
            }
        }
    
        return_paths
    }
    pub async fn select_path(
        &mut self,
        paths: &[Vec<H160>],
        amount_in: ethers::types::U256,
    ) -> (Vec<Trade>, U256) {
        let mut best_path = Vec::new();
        let mut best_value = U256::from(0);

        for path in paths {
            let (path_trades, value) = self.calculate_path(path, amount_in).await;
            if value > best_value {
                best_value = value;
                best_path = path_trades;
            }
        }

        (best_path, best_value)
    }

    pub async fn select_pool(
        &self,
        from: &H160,
        to: &H160,
        amount_in: U256,
    ) -> Option<(H160, Trade)> {
        //println!("Selecionando pool from {} to {}", from, to);

        let node_idx = match self.tokens_lookup.get(from) {
            Some(x) => x,
            None => {
                println!("Token não encontrado");
                return None;
            }
        };
        let node = self.tokens[*node_idx].read().await;

        let pools_with = match node.pools_by_pair.get(to) {
            Some(x) => x.clone(),
            None => {
                println!("Pool não encontrada");
                return None;
            }
        };
        // println!("pools with {:?}", &pools_with);
        let mut best_trade = None;

        for pool_addr in pools_with {
            if let Some(pool) = node.pools.get(&pool_addr) {
                let pool_guard = pool.pool.read().await;
                if let Some(trade) = pool_guard.trade(amount_in, pool.is0) {
                    //println!(
                    //    "Trade encontrada: {} -> {}: {} from0: {}",
                    //    trade.token0, trade.token1, trade.amount_out,trade.from0
                    // );
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
                    let p = pool.pool.read().await;
                    println!(
                        "Trade falhou em {} {} {}",
                        p.get_dex(),
                        p.get_version(),
                        p.get_fee()
                    );
                }
            } else {
                println!("Pool não encontrada para o endereço {}", pool_addr);
            }
        }

        best_trade
    }
    pub async fn calculate_path(&mut self, path: &[H160], amount_in: U256) -> (Vec<Trade>, U256) {
        let mut current_amount = amount_in;
        let mut trade_sequence = Vec::new();

        for i in 0..path.len() - 1 {
            let from_node = &path[i];
            let to_node = &path[i + 1];

            if let Some((pool_addr, mut trade)) =
                self.select_pool(from_node, to_node, current_amount).await
            {
                // Registrar a direção real usada
                trade.from0 = self.tokens_lookup.get(from_node).unwrap()
                    == self.tokens_lookup.get(&trade.token0).unwrap();
                trade_sequence.push(trade.clone());
                current_amount = trade.amount_out;
            } else {
                return (vec![], U256::zero());
            }
        }

        (trade_sequence, current_amount)
    }

    fn get_node_by_address(&self, address: &H160) -> Option<Arc<RwLock<Token>>> {
        let idx = match self.tokens_lookup.get(address) {
            Some(idx) => *idx,
            None => return None,
        };
        Some(self.tokens[idx].clone())
    }
}
