use std::path::PathBuf;

use sled::{Db, Tree};

use crate::sled_pool_key::SledPoolKey;

#[derive(Clone)]
pub struct ChainsDB {
    pub tokens_tree: Tree,
    pub pools_tree: Tree,
    pub dexes_tree: Tree,
    pub db: Db,
}

impl ChainsDB {
    fn get_pool(&self, key: SledPoolKey) {
        if let Ok(k) = bincode::encode_to_vec(key, bincode::config::standard()) {
            self.pools_tree.get(k);
        }
    }

    fn new(root_path: PathBuf) -> Result<ChainsDB, ()> {
        let db_path = root_path;

        let Ok(db) = sled::open(db_path) else {
            return Err(());
        };

        let Ok(tokens_tree) = db.open_tree("tokens") else {
            return Err(());
        };

        let Ok(dexes_tree) = db.open_tree("dexes") else {
            return Err(());
        };

        let Ok(pools_tree) = db.open_tree("pools") else {
            return Err(());
        };

        Ok(Self {
            tokens_tree,
            pools_tree,
            dexes_tree,
            db,
        })
    }
}
