use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use ethers::contract::Contract;
use ethers::middleware::transformer::ds_proxy::factory;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Block, Chain, H160, H256, U256, U64};

use futures::StreamExt;
use serde_json::Value;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

// We still use `SelectAll` and `FuturesUnordered` from `futures`:
// there is no direct Tokio equivalent for merging arbitrary streams
use futures::stream::{FuturesUnordered, SelectAll};

use crate::block_service::BlockService;
use crate::chain_graph::ChainGraph;
use crate::err::PoolUpdateError;
use crate::mult_provider::MultiProvider;
use crate::v2_pool_src::V2PoolSrc;
use crate::v3_pool_src::V3PoolSrc;
use crate::v_pool_src::AnyPoolSrc;
use crate::{factory::AnyFactory, pair::Pair, token::Token, AbisData};

/// A container that holds a Vec of (address, value) pairs and a lookup map from address to index.
/// Ensures both structures stay in sync via its API.
pub struct MappedVec<V> {
    entries: Vec<(H160, V)>,
    lookup: HashMap<H160, usize>,
}

impl<V> MappedVec<V> {
    /// Creates an empty MappedVec.
    pub fn new() -> Self {
        MappedVec {
            entries: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    /// Inserts a value for the given address.
    /// If the address was already present, replaces the old value and returns it.
    pub fn insert(&mut self, addr: H160, value: V) -> Option<V> {
        if let Some(&idx) = self.lookup.get(&addr) {
            // replace existing
            let (_, old) = std::mem::replace(&mut self.entries[idx], (addr, value));
            Some(old)
        } else {
            // push new
            let idx = self.entries.len();
            self.entries.push((addr, value));
            self.lookup.insert(addr, idx);
            None
        }
    }

    /// Returns a reference to the value for the given address, if any.
    pub fn get(&self, addr: &H160) -> Option<&V> {
        self.lookup.get(addr).map(|&i| &self.entries[i].1)
    }

    /// Returns a mutable reference to the value for the given address, if any.
    pub fn get_mut(&mut self, addr: &H160) -> Option<&mut V> {
        self.lookup
            .get(addr)
            .and_then(|&i| self.entries.get_mut(i))
            .map(|(_, v)| v)
    }

    /// Remove the value for the given address, if present.
    /// Uses swap_remove to keep the Vec compact and updates the moved element's index.
    pub fn remove(&mut self, addr: &H160) -> Option<V> {
        if let Some(idx) = self.lookup.remove(addr) {
            let (_removed_addr, removed_val) = self.entries.swap_remove(idx);
            // if we swapped another element into idx, update its lookup entry
            if let Some((moved_addr, _)) = self.entries.get(idx) {
                self.lookup.insert(*moved_addr, idx);
            }
            Some(removed_val)
        } else {
            None
        }
    }

    /// Returns an iterator over all stored values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over all stored (address, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = &(H160, V)> {
        self.entries.iter()
    }
}

/// Observes on-chain state: pools, tokens, and factories.
pub struct ChainSrc {
    provider: Arc<Provider<MultiProvider>>,
    pools: MappedVec<Arc<RwLock<AnyPoolSrc>>>,
    tokens: MappedVec<Arc<RwLock<Token>>>,
    factories: MappedVec<Arc<RwLock<AnyFactory>>>,

    ws_urls: Vec<String>,

    block_services: Arc<BlockService>,
    graph: ChainGraph,
    abis: Arc<AbisData>, //one struct with all abis
}

impl ChainSrc {
    /// Creates a new, empty observer.
    pub async fn new(
        abis: Arc<AbisData>,
        provider: Arc<Provider<MultiProvider>>,
        ws_urls: Vec<String>,
        tokens_list: Vec<Arc<RwLock<Token>>>,
        factories_list: Vec<Arc<RwLock<AnyFactory>>>,
    ) -> Self {
        println!("Creating ChainSrc");

        let mut src = ChainSrc {
            provider,
            pools: MappedVec::new(),
            tokens: MappedVec::new(),
            factories: MappedVec::new(),

            ws_urls: ws_urls.clone(),
            graph: ChainGraph {
                pools_by_token: HashMap::new(),
            },
            abis,
            block_services: Arc::new(BlockService::new(ws_urls, HashSet::new())),
        };

        // Populate tokens map
        for token in tokens_list {
            let addr = token.read().await.address;
            src.tokens.insert(addr, token.clone());
            println!("Token added: {}", token.read().await.symbol);
        }

        // Populate factories map
        for factory in factories_list {
            let addr = factory.read().await.get_address();
            src.factories.insert(addr, factory.clone());
            println!(
                "Factory added: {} {}",
                factory.read().await.get_name(),
                factory.read().await.get_version()
            );
        }

        src.discover_all_pools().await.unwrap();

        src
    }

    pub fn spawn_listener(block_rx: UnboundedReceiver<Block<H256>>) -> JoinHandle<()> {
        tokio::spawn(async move {
            println!("▶️ Block listener started");
            let mut block_rx = block_rx;

            while let Some(block) = block_rx.recv().await {
                if let Some(n) = block.number {
                    println!("🚀 New block #{}", n);
                }
            }

            println!("🔴 Block listener ended");
        })
    }
    /// Spawns the block‐listener in its own task and returns a handle you can abort or await.
    /// You can still use the original `Arc<ChainSrc>` afterward.
   
    pub async fn update_all(&mut self) {
        let pools = self.pools.values().cloned().collect::<Vec<_>>();
        let semaphore = Arc::new(Semaphore::new(50)); // Limit concurrent updates
        let mut updates = Vec::with_capacity(pools.len());

        for pool in pools {
            let semaphore = semaphore.clone();
            updates.push(async move {
                let _permit = semaphore.acquire().await;
                let mut guard = pool.write().await;
                guard.update().await
            });
        }

        let mut stream = futures::stream::iter(updates).buffer_unordered(50);
        while let Some(result) = stream.next().await {
            match result {
                Ok(addr) => println!("Updated pool {}", addr),
                Err(e) => println!("Update error: {:?}", e),
            }
        }
    }
    /// Discover and register all pools for every token pair in `tokens`.
    pub async fn discover_all_pools(&mut self) -> Result<(), PoolUpdateError> {
        // 1. Collect token addresses for all registered tokens
        let token_addrs: Vec<H160> = FuturesUnordered::from_iter(self.tokens.values().map(|tok| {
            let t = tok.clone();
            async move { t.read().await.address }
        }))
        .collect()
        .await;

        let pairs = Self::generate_unique_pairs(&token_addrs);

        // 2. Iterate unique pairs (i < j)
        for i in 0..token_addrs.len() {
            for j in (i + 1)..token_addrs.len() {
                let t0 = token_addrs[i];
                let t1 = token_addrs[j];

                // search and insert pools
                let found = self.search_pools(&t0, &t1).await;

                for pool_addr in found {
                    // after insertion, retrieve the pool to inspect direction
                    if let Some(pool_arc) = self.pools.get(&pool_addr) {
                        let is0 = pool_arc.read().await.is_0(&t0).await;
                        // update graph for both tokens
                        self.graph
                            .pools_by_token
                            .entry(t0)
                            .or_default()
                            .push((pool_addr, is0));
                        self.graph
                            .pools_by_token
                            .entry(t1)
                            .or_default()
                            .push((pool_addr, !is0));
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_unique_pairs(tokens: &[H160]) -> Vec<(H160, H160)> {
        let mut pairs = Vec::new();
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                pairs.push((tokens[i], tokens[j]));
            }
        }
        pairs
    }

    pub async fn search_pools(&mut self, token0: &H160, token1: &H160) -> Vec<H160> {
        let mut founded_pools = Vec::new();
        for factory in self.factories.values() {
            let (name, version, fees) = {
                let factory_read = factory.read().await;
                (
                    factory_read.get_name().to_string(),
                    factory_read.get_version().to_string(),
                    factory_read.supported_fees().to_vec(),
                )
            };
            for fee in fees {
                let maybe_pool = {
                    let mut factory_write = factory.write().await;
                    factory_write.get_pool(token0, token1, &fee, &0).await
                };

                if let Some(va) = maybe_pool {
                    founded_pools.push(va);

                    let pool_version = {
                        let factory_read = factory.read().await;
                        factory_read.get_version().to_string()
                    };

                    if pool_version == "v2" {
                        let v2_pool_contract =
                            Contract::new(va, self.abis.v2_pool.clone(), self.provider.clone());

                        let new_v2_pool = V2PoolSrc {
                            address: va,
                            token0: self.tokens.get(token0).unwrap().clone(),
                            token1: self.tokens.get(token1).unwrap().clone(),
                            exchange: factory.read().await.get_name().to_string(),
                            version: factory.read().await.get_version().to_string(),
                            fee,
                            reserves0: U256::from(0),
                            reserves1: U256::from(0),
                            contract: v2_pool_contract,
                        };

                        let new_any_pool = AnyPoolSrc::V2 { 0: new_v2_pool };

                        let pool_arc = Arc::new(RwLock::new(new_any_pool));
                        self.pools.insert(va, pool_arc.clone());
                    } else if pool_version == "v3" {
                        let v3_pool_contract =
                            Contract::new(va, self.abis.v3_pool.clone(), self.provider.clone());

                        let new_v3_pool = V3PoolSrc::new(
                            va,
                            self.get_token(token0).cloned().expect("Token0 not found"),
                            self.get_token(token1).cloned().expect("Token1 not found"),
                            factory.read().await.get_name().to_string(),
                            factory.read().await.get_version().to_string(),
                            fee,
                            v3_pool_contract,
                        )
                        .await;

                        let new_any_pool = AnyPoolSrc::V3 { 0: new_v3_pool };

                        let pool_arc = Arc::new(RwLock::new(new_any_pool));
                        self.pools.insert(va, pool_arc.clone());
                    }
                }
            }
        }
        founded_pools
    }

    /// Inserts or updates a token.
    pub async fn add_token(&mut self, token: Arc<RwLock<Token>>) {
        let addr = token.read().await.address;
        self.tokens.insert(addr, token.clone());
    }

    /// Inserts or updates a factory (DEX).
    pub async fn add_factory(&mut self, dex: Arc<RwLock<AnyFactory>>) {
        let addr = dex.read().await.get_address();
        self.factories.insert(addr, dex.clone());
    }

    /// Retrieves a pool by address.
    pub fn get_pool(&self, addr: &H160) -> Option<&Arc<RwLock<AnyPoolSrc>>> {
        self.pools.get(addr)
    }

    /// Retrieves a token by address.
    pub fn get_token(&self, addr: &H160) -> Option<&Arc<RwLock<Token>>> {
        self.tokens.get(addr)
    }

    /// Retrieves a factory by address.
    pub fn get_factory(&self, addr: &H160) -> Option<&Arc<RwLock<AnyFactory>>> {
        self.factories.get(addr)
    }
}
