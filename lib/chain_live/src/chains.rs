use std::collections::{BTreeMap, HashMap};

use alloy_provider::Provider;
use chain_db::{chains_db::ChainsDB, p_config::AnyPoolConfig};
use chain_json::chains::ChainsJsonInput;

use crate::{
    MyProvider,
    chain::Chain,
    generate_fallback_p, generate_fallback_provider,
    search_context::{self, SearchContext},
};

pub struct Chains {
    chains: BTreeMap<u64, Chain<'static, MyProvider>>,
    db: ChainsDB,
    ctx: SearchContext,
}

impl Default for Chains {
    fn default() -> Self {
        let default_data = ChainsJsonInput::default();
        let default_db = ChainsDB::default().unwrap();
        let default_ctx = SearchContext::default();
        let mut chains = BTreeMap::new();

        let mut s = Self {
            db: default_db,
            ctx: default_ctx,
            chains,
        };

        for (id, chain) in default_data.chains.iter() {
            if let Some(provider) = generate_fallback_provider(chain.http_nodes_urls.clone()) {
                let n_chain = Chain::new(&chain, provider, &s.ctx, *id);
            };
        }

        s
    }
}
