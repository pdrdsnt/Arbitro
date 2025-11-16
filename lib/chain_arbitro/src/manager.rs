use std::collections::BTreeMap;

use alloy::providers::Provider;
use chain_db::chains_db::ChainsDB;
use chain_json::chains::Chains;

use crate::chain_arbitro::ChainArbitro;

pub struct ArbitroManager<P: Provider + Clone> {
    chains: BTreeMap<u64, ChainArbitro<P>>,
}

impl<P: Provider + Clone> ArbitroManager<P> {
    pub fn new(chains: Chains, provider: P) -> Self {
        let map = BTreeMap::new();

        if let Ok(db_path) = PathBuf::from_str("../../chains_db/sled_db") {
            let local_db = ChainsDB::try_from(db_path);

            for (id, chain) in chains.chains {
                let new = ChainArbitro::from_chain(&chain, provider.clone(), local_db);
            }
        }

        Self { chains: map }
    }
}
