use std::{any::Any, collections::HashSet, path::PathBuf, task::Context};

use bincode::{Decode, Encode, config::Configuration, de::BorrowDecoder};
use sled::{Db, Tree};

use crate::{
    sled_pool_key::{SledPairKey, SledPoolCollection, SledPoolKey},
    sled_pool_parts::{AnyPoolConfig, AnyPoolLiquidityNets, AnyPoolState, PoolTokens},
};

#[derive(Clone)]
pub struct ChainsDB {
    config_by_pool: Tree,
    state_by_pool: Tree,
    tokens_by_pool: Tree,
    ticks_by_pool: Tree,
    pool_by_pair: Tree,
    db: Db,
}

impl ChainsDB {
    pub fn get_pool_state(&self, key: &SledPoolKey) -> Result<AnyPoolState, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }
    pub fn get_pool_tokens(&self, key: &SledPoolKey) -> Result<PoolTokens, sled::Error> {
        self.get_thing(&self.tokens_by_pool, key)
    }
    pub fn get_pool_config(&self, key: &SledPoolKey) -> Result<AnyPoolConfig, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }
    pub fn get_pool_ticks(&self, key: &SledPoolKey) -> Result<AnyPoolLiquidityNets, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }
    pub fn get_pools_with_pair(
        &self,
        key: &SledPairKey,
    ) -> Result<SledPoolCollection, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }

    pub fn save_pair_pool(
        &self,
        key: &SledPairKey,
        collection: SledPoolCollection,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        if let Ok(mut is) = self.get_pools_with_pair(key) {
            is.inner.extend(collection.inner);
            self.save_thing(&self.state_by_pool, key, &is)
        } else {
            self.save_thing(&self.state_by_pool, key, &collection)
        }
    }

    pub fn save_pool_state(
        &self,
        key: &SledPoolKey,
        state: &AnyPoolState,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.save_thing(&self.state_by_pool, key, state)
    }

    pub fn save_pool_config(
        &self,
        key: &SledPoolKey,
        state: &AnyPoolConfig,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.save_thing(&self.state_by_pool, key, state)
    }

    pub fn save_pool_tokens(
        &self,
        key: &SledPoolKey,
        state: &PoolTokens,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.save_thing(&self.state_by_pool, key, state)
    }

    pub fn save_pool_ticks(
        &self,
        key: &AnyPoolLiquidityNets,
        state: &AnyPoolState,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.save_thing(&self.state_by_pool, key, state)
    }
}

impl TryFrom<PathBuf> for ChainsDB {
    type Error = ();

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let db_path = path;

        let Ok(db) = sled::open(db_path) else {
            return Err(());
        };

        let Ok(p_configs) = db.open_tree("pool_configs") else {
            return Err(());
        };

        let Ok(p_states) = db.open_tree("pool_states") else {
            return Err(());
        };

        let Ok(p_tokens) = db.open_tree("pool_tokens") else {
            return Err(());
        };

        let Ok(p_ticks) = db.open_tree("pool_ticks") else {
            return Err(());
        };

        let Ok(p_by_pairs) = db.open_tree("pool_by_pair") else {
            return Err(());
        };

        Ok(Self {
            db,
            state_by_pool: p_states,
            tokens_by_pool: p_tokens,
            ticks_by_pool: p_ticks,
            config_by_pool: p_configs,
            pool_by_pair: p_by_pairs,
        })
    }
}
impl SledBincodeDb for ChainsDB {}

pub trait SledBincodeDb {
    fn get_thing<R: bincode::Decode<()>>(
        &self,
        tree: &Tree,
        key: &impl Encode,
    ) -> Result<R, sled::Error> {
        let key = bincode::encode_to_vec(key, bincode::config::standard())
            .map_err(|_| sled::Error::Unsupported("invalid key encoding".into()))?;

        let data = tree
            .get(key)?
            .ok_or_else(|| sled::Error::Unsupported("key not found".into()))?;

        let (decoded, _len): (R, usize) =
            bincode::decode_from_slice(&data, bincode::config::standard())
                .map_err(|_| sled::Error::Unsupported("invalid data decoding".into()))?;

        Ok(decoded)
    }

    fn save_thing(
        &self,
        tree: &Tree,
        key: &impl Encode,
        value: &impl Encode,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        let key = bincode::encode_to_vec(key, bincode::config::standard())
            .map_err(|_| sled::Error::Unsupported("invalid key encoding".into()))?;

        let data = bincode::encode_to_vec(value, bincode::config::standard())
            .map_err(|_| sled::Error::Unsupported("invalid value encoding".into()))?;

        tree.insert(key, data)
    }
}
