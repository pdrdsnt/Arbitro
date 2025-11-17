use chain_db::{chains_db::ChainsDB, p_config::AnyPoolConfig, p_key::SledPairKey};
use std::{collections::HashSet, sync::Arc};

pub struct SearchContext {
    known: HashSet<AnyPoolConfig>,
    tried: HashSet<AnyPoolConfig>,
}

impl Default for SearchContext {
    fn default() -> Self {
        let known = HashSet::new();
        let tried = HashSet::new();

        Self { known, tried }
    }
}

impl SearchContext {
    pub fn check(&self, pool: AnyPoolConfig) -> Result<AnyPoolConfig, AnyPoolConfig> {
        if self.known.contains(&pool) {
            return Err(pool);
        }

        if self.tried.contains(&pool) {
            return Err(pool);
        }

        Ok(pool)
    }
}
