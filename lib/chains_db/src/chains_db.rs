use std::path::PathBuf;

use bincode::config::Configuration;
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
        if let Ok(k) = bincode::encode_to_vec(key, bincode::config::standard()) {
            match self.state_by_pool.get(k) {
                Ok(value) => todo!(),
                Err(err) => return Err(err),
            }
            if let Ok(state) = self.state_by_pool.get(k) {
                if let Some(data) = state {
                    let decoded = bincode::decode_from_slice::<AnyPoolState, Configuration>(
                        &data,
                        bincode::config::standard(),
                    );
                } else {
                    return Err(sled::Error::Unsupported("value not founded".to_string()));
                }
            }
        }
        Err(sled::Error::Unsupported("key not valid".to_string()))
    }

    pub fn save_pool_state(&self, key: SledPoolKey, state: AnyPoolState) {
        if let (Ok(k), Ok(d)) = (
            bincode::encode_to_vec(key, bincode::config::standard()),
            bincode::encode_to_vec(state, bincode::config::standard()),
        ) {
            self.state_by_pool.insert(k, d);
        }
    }

    fn new(root_path: PathBuf) -> Result<ChainsDB, ()> {
        let db_path = root_path;

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
