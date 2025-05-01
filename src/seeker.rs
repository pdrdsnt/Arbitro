use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use ethers::{middleware::transformer::ds_proxy::factory, types::H160};

use crate::{factory::AnyFactory, pool_utils::AnyPool, token::Token};
/// A container that holds a Vec of (address, value) pairs and a lookup map from address to index.
/// Ensures both structures stay in sync via its API.
pub struct MappedVec<V> {
    entries: Vec<(H160, V)>,
    lookup: HashMap<H160, usize>,
}

impl<V> MappedVec<V> {
    /// Creates an empty MappedVec.
    pub fn new() -> Self {
        MappedVec { entries: Vec::new(), lookup: HashMap::new() }
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
        self.lookup.get(addr).map(|&i| unsafe { &mut *(&mut self.entries[i].1 as *mut V) })
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
    pub fn values(&self) -> impl Iterator<Item=&V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over all stored (address, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item=&(H160, V)> {
        self.entries.iter()
    }
}

/// Observes on-chain state: pools, tokens, and factories.
pub struct ChainObserver {
    pools: MappedVec<Arc<RwLock<AnyPool>>>,
    tokens: MappedVec<Arc<RwLock<Token>>>,
    factories: MappedVec<Arc<RwLock<AnyFactory>>>,
}

impl ChainObserver {
    /// Creates a new, empty observer.
    pub fn new() -> Self {
        ChainObserver {
            pools: MappedVec::new(),
            tokens: MappedVec::new(),
            factories: MappedVec::new(),
        }
    }

    pub async fn search_pools(&self, token0: &H160,token1: &H160) -> Vec<H160> {
        let mut founded_pools = Vec::new();
        for factory in self.factories.values() {
            let fees = {
                let factory_read = factory.read().unwrap();
                factory_read.supported_fees().to_vec()
            };
            for fee in fees
            {
                if let Some(va) = factory.write().unwrap().get_pool(token0, token1, &fee, &20).await {
                    founded_pools.push(va);
                }
        
            }
        }
        founded_pools
    }

    /// Inserts or updates a pool.
    /// Records the pool itself, which tokens it uses, and registers it under each token.
    pub fn add_pool(&mut self, pool: Arc<RwLock<AnyPool>>) {
        let pool_read = pool.read().unwrap();
        let addr = pool_read.get_address();
        let [tok0, tok1] = pool_read.get_tokens();

        // 1) add to main pool list
        self.pools.insert(addr, pool.clone());
    }

    /// Inserts or updates a token.
    pub fn add_token(&mut self, token: Arc<RwLock<Token>>) {
        let addr = token.read().unwrap().address;
        self.tokens.insert(addr, token.clone());
    }

    /// Inserts or updates a factory (DEX).
    pub fn add_factory(&mut self, dex: Arc<RwLock<AnyFactory>>) {
        let addr = dex.read().unwrap().get_address();
        self.factories.insert(addr, dex.clone());
    }

    /// Retrieves a pool by address.
    pub fn get_pool(&self, addr: &H160) -> Option<&Arc<RwLock<AnyPool>>> {
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
