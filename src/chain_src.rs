use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::http::uri;
use ethers::{
    abi,
    contract::Contract,
    middleware::transformer::ds_proxy::factory,
    providers::{Middleware, Provider, Ws},
    types::{Block, Chain, Filter, Log, H160, H256, U256, U64},
};
use ethers_providers::Http;
// We still use `SelectAll` and `FuturesUnordered` from `futures`:
// there is no direct Tokio equivalent for merging arbitrary streams
use futures::stream::{FuturesUnordered, SelectAll};
use futures::{future::join_all, SinkExt, StreamExt};
use serde_json::Value;
use tokio::{
    sync::{mpsc, mpsc::UnboundedReceiver, watch, Mutex, RwLock, RwLockReadGuard, Semaphore},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    arbitro::Arbitro,
    blockchain_db::{DexModel, TokenModel},
    chain_graph::ChainGraph,
    chain_svc::ChainDataService,
    err::PoolUpdateError,
    factory::{AnyFactory, Factory},
    mapped_vec::{self, MappedVec},
    mult_provider::MultiProvider,
    pair::Pair,
    token::Token,
    v2_pool_sim::V2PoolSim,
    v2_pool_src::V2PoolSrc,
    v3_pool_sim::V3PoolSim,
    v3_pool_src::V3PoolSrc,
    v_pool_sim::AnyPoolSim,
    v_pool_src::AnyPoolSrc,
    AbisData,
};

/// Observes on-chain state: pools, tokens, and factories.
pub struct ChainSrc {
    provider: Arc<Provider<MultiProvider,>,>,
    pools: MappedVec<Arc<RwLock<AnyPoolSrc,>,>,>,
    pub tokens: MappedVec<Arc<RwLock<Token,>,>,>,
    factories: MappedVec<Arc<RwLock<AnyFactory,>,>,>,
    abis: Arc<AbisData,>, // one struct with all abis
}

impl ChainSrc {
    /// Creates a new, empty observer.
    pub async fn new(
        abis: Arc<AbisData,>, provider: Arc<Provider<MultiProvider,>,>, tokens_list: &Vec<TokenModel,>,
        _dexes: &Vec<DexModel,>,
    ) -> Self {
        println!("Creating ChainSrc");
        /* tokens will be add one by one
        the time it takes to create all pools we lost events
        let tokens = {
            let mut tokens: Vec<(H160, Arc<RwLock<Token>>)> = Vec::new();
            for t in tokens_list {
                let add = H160::from_str(&t.address,).unwrap();
                let token = Token::new(
                    t.name.clone(),
                    add.clone(),
                    t.symbol.clone(),
                    t.decimals,
                    Contract::new(add, abis.v2_factory.clone(), provider.clone(),),
                    // PLACEHOLDER WE DONT CALL TOKEN FUNCTION SO DOESNT MATTER
                );
                tokens.push((add, Arc::new(RwLock::new(token,),),),);
            }
            tokens
        };
        */
        let mut factories = {
            let mut _factories = Vec::new();
            for d in _dexes {
                let add = H160::from_str(&d.factory,).unwrap();

                if d.version == "v2" {
                    let factory = Factory::new(
                        d.dex_name.clone(),
                        Contract::new(add, abis.v2_factory.clone(), provider.clone(),),
                    );
                    let v2_factory = AnyFactory::V2(factory,);
                    _factories.push((add, Arc::new(RwLock::new(v2_factory,),),),);
                } else if d.version == "v3" {
                    let factory = Factory::new(
                        d.dex_name.clone(),
                        Contract::new(add, abis.v3_factory.clone(), provider.clone(),),
                    );
                    let v3_factory = AnyFactory::V3(factory,);
                    _factories.push((add, Arc::new(RwLock::new(v3_factory,),),),);
                }
            }
            _factories
        };

        let mut src = ChainSrc {
            provider,
            pools: MappedVec::new(),
            tokens: MappedVec::new(), //MappedVec::from_array(tokens,),
            factories: MappedVec::from_array(factories,),
            abis,
        };

        /* nope
        src.search_all_pools().await.unwrap();
        */
        src
    }

    pub async fn create_sim(&self,) -> Vec<(H160, AnyPoolSim,),> {
        let futures = self.pools.iter().map(|(addr, pool,)| {
            let addr = addr.clone(); // clone H160
            let pool = pool.clone(); // clone Arc<RwLock<...>> or similar
            async move {
                let lock = pool.read().await;
                let sim = lock.into_sim().await;
                (addr, sim,)
            }
        },);

        join_all(futures,).await
    }

    pub async fn snapshot_tokens(&self,) -> MappedVec<Token,> {
        let mut mapped_vec = MappedVec::new();
        for t in self.tokens.values() {
            let tkn = t.read().await;
            mapped_vec.insert(tkn.address.clone(), tkn.clone(),);
        }
        mapped_vec
    }

    pub async fn update_all(&mut self,) {
        let pools = self.pools.values().cloned().collect::<Vec<_,>>();
        let semaphore = Arc::new(Semaphore::new(50,),); // Limit concurrent updates
        let mut updates = Vec::with_capacity(pools.len(),);

        for pool in pools {
            let semaphore = semaphore.clone();
            updates.push(async move {
                let _permit = semaphore.acquire().await;
                let mut guard = pool.write().await;
                guard.update().await
            },);
        }

        let mut stream = futures::stream::iter(updates,).buffer_unordered(50,);
        while let Some(result,) = stream.next().await {
            match result {
                Ok(addr,) => println!("Updated pool {}", addr),
                Err(e,) => println!("Update error: {:?}", e),
            }
        }
    }
    /// Discover and register all pools for every token pair in `tokens`.
    pub async fn search_all_pools(&mut self,) -> Result<(), PoolUpdateError,> {
        let token_addrs: Vec<H160,> = FuturesUnordered::from_iter(self.tokens.values().map(|tok| {
            let t = tok.clone();
            async move { t.read().await.address }
        },),)
        .collect()
        .await;

        let pairs = Self::generate_unique_pairs(&token_addrs,);

        let pools: Vec<_,> = pairs.iter().map(|(t0, t1,)| self.search_pools(t0, t1,),).collect();
        let r = join_all(pools,).await;
        for pool in r {
            for p in pool {
                let addr = p.address;
                if self.pools.get(&addr,).is_none() {
                    self.pools.insert(addr, p.pool.clone(),);
                }
            }
        }
        Ok((),)
    }

    fn generate_unique_pairs(tokens: &[H160],) -> Vec<(H160, H160,),> {
        let mut pairs = Vec::new();
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                pairs.push((tokens[i], tokens[j],),);
            }
        }
        pairs
    }

    pub async fn search_pools(&self, token0: &H160, token1: &H160,) -> Vec<DiscoveredPool,> {
        let mut tasks = FuturesUnordered::new();

        for factory in self.factories.values() {
            let (name, version, fees,) = {
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
                    factory_write.get_pool(token0, token1, &fee, &0,).await
                };

                if let Some(va,) = maybe_pool {
                    if let Some(a,) = self.pools.get(&va,) {
                        continue;
                    }
                    let pool_version = {
                        let factory_read = factory.read().await;
                        factory_read.get_version().to_string()
                    };
                    let f = Self::search_pool(
                        self.abis.clone(),
                        self.provider.clone(),
                        factory.clone(),
                        fee,
                        self.tokens.get(token0,).unwrap().clone(),
                        self.tokens.get(token1,).unwrap().clone(),
                    );

                    tasks.push(async move { f.await },);
                }
            }
        }
        let mut found = Vec::new();
        while let Some(maybe_pool,) = tasks.next().await {
            if let Some(pool,) = maybe_pool {
                found.push(pool,);
            }
        }

        found
    }

    pub async fn search_pool(
        abis: Arc<AbisData,>, provider: Arc<Provider<MultiProvider,>,>, factory: Arc<RwLock<AnyFactory,>,>, fee: u32,
        token0: Arc<RwLock<Token,>,>, token1: Arc<RwLock<Token,>,>,
    ) -> Option<DiscoveredPool,> {
        // Snapshot addresses (and fix the typo for t1)
        let t0 = token0.read().await.address.clone();
        let t1 = token1.read().await.address.clone();
        // Ask factory for the pool
        let maybe_pool = {
            let mut fac = factory.write().await;
            let res = fac.get_pool(&t0, &t1, &fee, &0,).await;
            res
        };

        // If no pool, bail early
        if maybe_pool.is_none() {
            return None;
        }

        // We have an address—figure out version & name
        let va = maybe_pool.unwrap();

        let (pool_version, exchange_name,) = {
            println!("  → acquiring read lock on factory to get version/name…");
            let fac = factory.read().await;
            (fac.get_version().to_string(), fac.get_name().to_string(),)
        };

        // Build the right pool source
        let pool_arc = if pool_version == "v2" {
            let v2_contract = Contract::new(va, abis.v2_pool.clone(), provider.clone(),);
            let new_v2 = V2PoolSrc {
                address: va,
                token0: token0.clone(),
                token0_addr: t0,
                token1: token1.clone(),
                token1_addr: t1,
                exchange: exchange_name.clone(),
                version: pool_version.clone(),
                fee,
                reserves0: U256::zero(),
                reserves1: U256::zero(),
                contract: v2_contract,
            };
            Arc::new(RwLock::new(AnyPoolSrc::V2 { 0: new_v2, },),)
        } else {
            let v3_contract = Contract::new(va, abis.v3_pool.clone(), provider.clone(),);
            let new_v3 = V3PoolSrc::new(
                va,
                token0.clone(),
                token1.clone(),
                exchange_name.clone(),
                pool_version.clone(),
                fee,
                v3_contract,
            )
            .await;
            Arc::new(RwLock::new(AnyPoolSrc::V3 { 0: new_v3, },),)
        };

        let discovered = DiscoveredPool {
            address: va,
            pool: pool_arc,
        };

        Some(discovered,)
    }

    /// Inserts or updates a token.
    pub async fn add_token(&mut self, token: Arc<RwLock<Token,>,>,) {
        let addr = token.read().await.address;
        self.tokens.insert(addr, token.clone(),);
    }

    /// Inserts or updates a factory (DEX).
    pub async fn add_factory(&mut self, dex: Arc<RwLock<AnyFactory,>,>,) {
        let addr = dex.read().await.get_address();
        self.factories.insert(addr, dex.clone(),);
    }

    /// Retrieves a pool by address.
    pub fn get_pool(&self, addr: &H160,) -> Option<&Arc<RwLock<AnyPoolSrc,>,>,> { self.pools.get(addr,) }

    /// Retrieves a token by address.
    pub fn get_token(&self, addr: &H160,) -> Option<&Arc<RwLock<Token,>,>,> { self.tokens.get(addr,) }

    /// Retrieves a factory by address.
    pub fn get_factory(&self, addr: &H160,) -> Option<&Arc<RwLock<AnyFactory,>,>,> { self.factories.get(addr,) }
}
pub struct DiscoveredPool {
    pub address: H160,
    pub pool: Arc<RwLock<AnyPoolSrc,>,>,
}
