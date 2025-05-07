use bigdecimal::BigDecimal;
use ethers::{
    abi::{token, Tokenizable},
    contract::Contract,
    core::k256::elliptic_curve::consts::{U20, U32},
    providers::{Http, Provider, Ws},
    types::{H160, U256},
    utils::hex::ToHex,
};

use futures::{future::join_all, stream::FuturesUnordered, StreamExt};
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
    arbitro, blockchain_db::{DexModel, TokenModel}, chain_observer::ChainObserver, dex::{self, AnyDex, Dex}, factory::{AnyFactory, Factory}, mult_provider::MultiProvider, pair::{self, Pair}, pool::{self, Pool}, pool_utils::{AbisData, AnyPool, PoolDir, Trade}, token::Token
};
use bigdecimal::ToPrimitive;
use futures::future;
use tokio::sync::{RwLock, Semaphore};

pub struct Arbitro {
    pub chain_observer: Arc<RwLock<ChainObserver>>,
}

impl Arbitro {
    pub async fn new(
        dexes_data: &Vec<DexModel>,
        tokens_data: &Vec<TokenModel>,
        provider: Arc<Provider<MultiProvider>>,
        abis: Arc<AbisData>,
    ) -> Self {
        let mut chain_observer = Arc::new(RwLock::new(ChainObserver::new()));

        for dex in dexes_data {
            let factory = match dex.version.as_str() {
                "v2" => {
                    let address_string = &dex.factory;
                    let address = H160::from_str(address_string).unwrap();
                    let contract =
                        Contract::new(address, abis.v2_factory.clone(), provider.clone());
                    AnyFactory::V2(Factory {
                        name: dex.dex_name.clone(),
                        factory: contract,
                        pools: Vec::new(),
                    })
                }
                "v3" => {
                    let address_string = &dex.factory;
                    let address = H160::from_str(address_string).unwrap();
                    let contract =
                        Contract::new(address, abis.v3_factory.clone(), provider.clone());
                    AnyFactory::V3(Factory {
                        name: dex.dex_name.clone(),
                        factory: contract,
                        pools: Vec::new(),
                    })
                }
                _ => panic!("Versão de DEX desconhecida"),
                
            };
        
        }

        for token in tokens_data {
            let address_string = &token.address;
            let address = H160::from_str(address_string).unwrap();
            let contract = Contract::new(address, abis.bep_20.clone(), provider.clone());
            let token = Token::new(
                token.name.clone(),
                address,
                token.symbol.clone(),
                token.decimals,
                contract,
                HashMap::new(),
            );
            
            chain_observer
                .write()
                .await
                .add_token(Arc::new(RwLock::new(token)));
        }
        
        chain_observer.write().await.create_pools().await;

        Arbitro {
            chain_observer,
        }
    
    }

    pub async fn arbitrage(&mut self, from: &H160, amount_in: U256) -> Vec<Vec<Trade>> {
        let forward_paths = self.find_foward_paths(from).await;
        let mut profitable_paths = Vec::new();
        
        for foward_paths in &forward_paths.get(&*from) {
            println!("--------------");
            
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
                //pool.pool.write().await.update().await;

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
