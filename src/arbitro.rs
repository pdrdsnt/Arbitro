use bigdecimal::BigDecimal;
use ethers::{
    abi::{token, Tokenizable},
    contract::Contract,
    core::k256::elliptic_curve::consts::{U20, U32},
    providers::{Http, Provider, Ws},
    types::{H160, U256},
    utils::hex::ToHex,
};

use futures::{stream::FuturesUnordered, StreamExt};
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
    blockchain_db::{DexModel, TokenModel}, dex::{self, AnyDex, Dex}, mult_provider::MultiProvider, pair::{self, Pair}, pool::{self, Pool, V2Pool}, pool_utils::{AbisData, AnyPool, PoolDir, Trade}, token::Token
};
use bigdecimal::ToPrimitive;
use futures::future;
use tokio::sync::{RwLock, Semaphore};

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
        provider: Arc<Provider<MultiProvider>>,
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
        provider: Arc<Provider<MultiProvider>>,
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
                    pools_by_address: HashMap::new(),
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
                    pools_by_address: HashMap::new(),
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
        provider: Arc<Provider<MultiProvider>>,
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
            let token_contract: ethers::contract::ContractInstance<Arc<Provider<MultiProvider>>, Provider<MultiProvider>> = Contract::new(addr, abis.bep_20.clone(), provider.clone());

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
        // 1) Gather all token addresses in one go:
        let tokens_addresses: Vec<H160> =
            FuturesUnordered::from_iter(self.tokens.iter().map(|tok| {
                let tok = tok.clone();
                async move { tok.read().await.address }
            }))
            .collect()
            .await;
            
        if tokens_addresses.len() < 2 {
            return;
        }
        let semaphore = Arc::new(Semaphore::new(6)); // Limit to 10 concurrent requests

        // 2) Prepare a task for every (token0, token1, dex, fee) combination:
        let mut tasks = FuturesUnordered::new();
        for (i, &addr0) in tokens_addresses
            .iter()
            .enumerate()
            .take(tokens_addresses.len() - 1)
        {
            for &addr1 in &tokens_addresses[i + 1..] {
                // lookup indices once
                let t0_idx = match self.tokens_lookup.get(&addr0) {
                    Some(&i) => i,
                    None => continue,
                };
                let t1_idx = match self.tokens_lookup.get(&addr1) {
                    Some(&i) => i,
                    None => continue,
                };

                // clone out everything we'll need in the spawned task
                let tokens = self.tokens.clone();
                let lookup = self.tokens_lookup.clone();
                let dexes = self.dexes.clone(); // assuming your dex type is `Clone`
                let token0_a = addr0;
                let token1_a = addr1;
                let sem = semaphore.clone();

                tasks.push(tokio::spawn(async move {
                    let _permit = sem.acquire_owned().await.unwrap();
                    // re‑read the token structs
                    let token0 = tokens[t0_idx].read().await.clone();
                    let token1 = tokens[t1_idx].read().await.clone();
                    let pair = Pair::new(token0.address, token1.address);

                    let mut found = Vec::new();
                    for mut dex in dexes {
                        for &fee in dex.supported_fees().iter() {
                            if let Some(pool) =
                                dex.get_pool(pair.clone(), &tokens, &lookup, fee).await
                            {
                                // record (pool, from0, idx0, idx1)
                                let from0 = pair.a == token0.address;
                                found.push((pool, from0, t0_idx, t1_idx));
                            }
                        }
                    }
                    found
                }));
            }
        }

        // Consume tasks with results
        while let Some(Ok(results)) = tasks.next().await {
            for (pool, from0, t0_idx, t1_idx) in results {
                let mut t0 = self.tokens[t0_idx].write().await;
                let mut t1 = self.tokens[t1_idx].write().await;
                t0.add_pool(pool.clone(), from0).await;
                t1.add_pool(pool, !from0).await;
            }
        }
    }

    pub async fn arbitrage(&mut self, from: &H160, amount_in: U256) -> Vec<Vec<Trade>> {
        let forward_paths = self.find_foward_paths(from).await;
        let mut profitable_paths = Vec::new();
        println!(
            "initializing arbitrage for {:?}",
            self.get_symbol(from).await
        );
        for foward_paths in &forward_paths.get(&*from) {
            println!("--------------");
            println!(
                "procurando caminhos melhor ciclo para {} -> ",
                self.get_symbol(from).await
            );
            println!("--------------");
            if let Some((best_foward, foward_amount)) =
                self.select_best_foward(foward_paths, amount_in).await
            {
                let return_amount = match best_foward.last() {
                    Some(amount) => amount.amount_out,
                    None => {
                        println!("Nenhum valor retornado");
                        continue;
                    }
                };

                self.print_path("path: ", &best_foward).await;
                let profit = return_amount.checked_sub(amount_in);
                if profit.is_some() {
                    profitable_paths.push(best_foward);
                    println!("Profit found: {}", profit.unwrap());
                } else {
                    let p = amount_in.saturating_sub(return_amount);
                    println!("No profit found: -{}", p);
                }
            }
        }

        profitable_paths
        // self.process_profitable_paths(profitable_paths).await;
    }

    async fn select_best_foward(
        &mut self,
        paths: &[Vec<H160>],
        amount_in: U256,
    ) -> Option<(Vec<Trade>, U256)> {
        let mut best_amount = U256::zero();
        let mut best_path = Vec::new();

        for path in paths {
            let (trades, amount_out) = self.calculate_path(path, amount_in).await;
            if amount_out > best_amount {
                best_amount = amount_out;
                best_path = trades;
            }
        }

        if best_amount > U256::zero() {
            Some((best_path, best_amount))
        } else {
            None
        }
    }

    async fn select_best_return(
        &mut self,
        paths: Vec<Vec<H160>>,
        amount_in: U256,
    ) -> Option<(Vec<Trade>, U256)> {
        let mut best_amount = U256::zero();
        let mut best_path = Vec::new();

        for return_path in paths {
            let (trades, amount_out) = self.calculate_path(&return_path, amount_in).await;
            if amount_out > best_amount {
                best_amount = amount_out;
                best_path = trades;
            }
        }

        if best_amount > U256::zero() {
            Some((best_path, best_amount))
        } else {
            None
        }
    }

    async fn process_profitable_paths(&self, paths: Vec<(U256, Vec<Trade>, Vec<Trade>)>) {
        for (profit, foward, return_path) in paths {
            println!("Lucro encontrado: {}", profit);
            self.print_path("Ida", &foward);
            self.print_path("Volta", &return_path);
            println!("=========================");
        }
    }

    async fn print_pool_data(&mut self, label: &str, pool_addr: &H160) {
        println!("--- {} ---", label);

        for mut dex in self.dexes.iter_mut() {
            if let Some(pool) = dex.get_pool_by_address(*pool_addr) {
                let pool_guard = pool.read().await;
                let tokens = pool_guard.get_tokens();
                println!(
                    "{} {} {}: {} -> {}",
                    pool_guard.get_dex(),
                    pool_guard.get_version(),
                    pool_guard.get_fee(),
                    tokens[0],
                    tokens[1]
                );
            } else {
                println!("Pool não encontrada");
            }
        }
    }

    async fn print_path(&self, label: &str, trades: &[Trade]) {
        println!("--- {} ---", label);
        for trade in trades {
            let from = if trade.from0 {
                &trade.token0
            } else {
                &trade.token1
            };
            let to = if trade.from0 {
                &trade.token1
            } else {
                &trade.token0
            };
            println!(
                "{} {} {} - {} {} -> {} {}",
                trade.dex,
                trade.version,
                trade.fee,
                trade.amount_in,
                self.get_symbol(from).await,
                self.get_symbol(to).await,
                trade.amount_out,
            );
            let _pool = self
                .get_node_by_address(from)
                .unwrap()
                .read()
                .await
                .pools
                .get(&trade.pool)
                .unwrap()
                .pool
                .clone();
            let pool = _pool.read().await;

            let token0 = pool.get_tokens()[0];
            let token1 = pool.get_tokens()[1];
            let tkn0_idx = self
                .tokens_lookup
                .get(&token0)
                .expect("Token0 não encontrado");
            let tkn1_idx = self
                .tokens_lookup
                .get(&token1)
                .expect("Token1 não encontrado");
            let token0 = self.tokens[*tkn0_idx].read().await;
            let token1 = self.tokens[*tkn1_idx].read().await;

            //println!("-- Pool data --");
            //println!("{:?}", pool.get_address());
            //println!("price: {}", pool.get_price([token0.decimals, token1.decimals]).unwrap());
            //println!("reserves {:?}", pool.get_reserves().unwrap());
        }
    }

    async fn get_symbol(&self, address: &H160) -> String {
        if let Some(idx) = self.tokens_lookup.get(address) {
            let token = self.tokens[*idx].read().await;
            token.symbol.clone()
        } else {
            "Unknown".to_string()
        }
    }

    async fn print_trade_flow(&self, trades: &[Trade]) {
        for trade in trades {
            let token0 = self.get_symbol(&trade.token0).await;
            let token1 = self.get_symbol(&trade.token1).await;

            println!(
                "{} {} → {}: {} (Fee: {}%) [Dex: {}]",
                trade.amount_in,
                if trade.from0 { &token0 } else { &token1 },
                if trade.from0 { &token1 } else { &token0 },
                trade.amount_out,
                trade.fee / 100, // Exibe fee como porcentagem
                trade.dex
            );
        }
    }

    fn path_uses_pool(&self, path: &[H160], pool_addr: &H160) -> bool {
        path.contains(pool_addr)
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

    pub async fn find_foward_paths(&self, start: &H160) -> HashMap<H160, Vec<Vec<H160>>> {
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

    /// Return all token‐sequences that start at `from`, end at `to`, within `max_depth` hops.
    pub async fn find_return_paths(
        &self,
        from: H160,
        to: H160,
        max_depth: usize,
    ) -> Vec<Vec<H160>> {
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        // begin at `from`
        queue.push_back(vec![from]);

        while let Some(path) = queue.pop_front() {
            // we count hops as edges, so path.len() > max_depth+1 => too deep
            if path.len() > max_depth + 1 {
                continue;
            }

            let last = *path.last().unwrap();
            // if we've come back to `to`, record the *full* path
            if last == to {
                results.push(path.clone());
                continue;
            }

            // otherwise, keep exploring
            for &next in &self.get_conected_nodes(&last).await {
                // avoid cycles
                if path.contains(&next) {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push(next);
                queue.push_back(new_path);
            }
        }

        results
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
        &mut self,
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
        let to_node_idx = match self.tokens_lookup.get(to) {
            Some(x) => x,
            None => {
                println!("Token não encontrado");
                return None;
            }
        };
        let to_node = self.tokens[*to_node_idx].read().await;
        let pools_with = match node.pools_by_pair.get(to) {
            Some(x) => x.clone(),
            None => {
                println!("Pool não encontrada");
                return None;
            }
        };
        //println!("{} pools {:?} with {}",node.symbol ,&pools_with, to_node.symbol);
        let mut best_trade = None;

        for pool_addr in pools_with {
            if let Some(pool) = node.pools.get(&pool_addr) {
                let pool_guard = pool.pool.read().await;
                //println!("-- {} --", pool_guard.get_address());
                //println!(
                //    "pool: {:?} {} {}",
                //    pool_guard.get_dex(),
                //    pool_guard.get_version(),
                //    pool_guard.get_fee()
                //);
                if let Some(trade) = pool_guard.trade(amount_in, pool.is0) {
                    let pool_addr_clone = pool_addr.clone();

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
                    //println!(
                    //    "Trade falhou em {} {} {}",
                    //    p.get_dex(),
                    //    p.get_version(),
                    //    p.get_fee()
                    //);
                }
            } else {
                println!("Pool não encontrada para o endereço {}", pool_addr);
            }
        }
        /*
               match &best_trade {
                   Some((pool_addr, trade)) => {
                       let p = match node.pools.get(&pool_addr) {
                           Some(p) => p.pool.read().await,
                           None => {
                               println!("Pool não encontrada");
                               return None;
                           }
                       };

                       println!(
                           "Trade: {} {} -> {} {}",
                           self.get_symbol(if trade.from0 {
                               &trade.token0
                           } else {
                               &trade.token1
                           })
                           .await,
                           trade.amount_in,
                           self.get_symbol(if trade.from0 {
                               &trade.token1
                           } else {
                               &trade.token0
                           })
                           .await,
                           trade.amount_out
                       );

                       println!("em {} {} {}", p.get_dex(), p.get_version(), p.get_fee());
                   }
                   None => {
                       println!("Nenhuma trade encontrada");
                       return None;
                   }
               }
        */
        best_trade
    }
    pub async fn calculate_path(&mut self, path: &[H160], amount_in: U256) -> (Vec<Trade>, U256) {
        //println!("Calculando caminho: {:?}", path);
        let mut current_amount = amount_in;
        let mut trade_sequence = Vec::new();

        for i in 0..path.len() - 1 {
            let from_node = &path[i];
            let to_node = &path[i + 1];

            if let Some((pool_addr, mut trade)) =
                self.select_pool(from_node, to_node, current_amount).await
            {
                trade.from0 = self.tokens_lookup.get(from_node).unwrap()
                    == self.tokens_lookup.get(&trade.token0).unwrap();
                trade_sequence.push(trade.clone());
                current_amount = trade.amount_out;
                //println!(
                //    "Trade: {} -> {}: {} ({} -> {})",
                //    self.get_symbol(&trade.token0).awa, self.get_symbol(&trade.token1).awa, trade.amount_out, trade.dex, trade.version
                //);
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
