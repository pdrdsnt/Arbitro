use std::{any::Any, path::PathBuf, task::Context};

use bincode::{Decode, Encode, config::Configuration, de::BorrowDecoder};
use sled::{Db, Tree};

use crate::{
    sled_pool_key::SledPoolKey,
    sled_pool_parts::{AnyPoolConfig, AnyPoolState, PoolTokens},
};

#[derive(Clone)]
pub struct ChainsDB {
    pub config_by_pool: Tree,
    pub state_by_pool: Tree,
    pub tokens_by_pool: Tree,
    pub ticks_by_pool: Tree,

    pub db: Db,
}

impl ChainsDB {
    pub fn get_pool_state(&self, key: SledPoolKey) -> Result<AnyPoolState, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }
    pub fn get_pool_tokens(&self, key: SledPoolKey) -> Result<PoolTokens, sled::Error> {
        self.get_thing(&self.tokens_by_pool, key)
    }
    pub fn get_pool_config(&self, key: SledPoolKey) -> Result<AnyPoolConfig, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }
    pub fn get_pool_ticks(&self, key: SledPoolKey) -> Result<AnyPool, sled::Error> {
        self.get_thing(&self.state_by_pool, key)
    }

    pub fn save_pool_state(
        &self,
        key: SledPoolKey,
        state: AnyPoolState,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.save_thing(&self.state_by_pool, key, state)
    }
}

impl TryFrom<PathBuf> for ChainsDB {
    type Error = ();

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let db_path = value;

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

        Ok(Self {
            db,
            state_by_pool: p_states,
            tokens_by_pool: p_tokens,
            ticks_by_pool: p_ticks,
            config_by_pool: p_configs,
        })
    }
}
impl SledBincodeDb for ChainsDB {}

pub trait SledBincodeDb {
    fn get_thing<R: bincode::Decode<()>>(
        &self,
        tree: &Tree,
        key: impl Encode,
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
        key: impl Encode,
        value: impl Encode,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        let key = bincode::encode_to_vec(key, bincode::config::standard())
            .map_err(|_| sled::Error::Unsupported("invalid key encoding".into()))?;

        let data = bincode::encode_to_vec(value, bincode::config::standard())
            .map_err(|_| sled::Error::Unsupported("invalid value encoding".into()))?;

        tree.insert(key, data)
    }
}
