use std::{path::PathBuf, str::FromStr};

use crate::chains_db::ChainsDB;

const DB_PATH: &str = "../.data";

impl ChainsDB {
    pub fn default() -> Option<Self> {
        if let Ok(path) = PathBuf::from_str(DB_PATH) {
            if let Ok(db) = ChainsDB::try_from(path) {
                return Some(db);
            }
        }

        None
    }
}
