use alloy_provider::Provider;
use chain_db::chains_db::ChainsDB;
use chain_json::chain::ChainJsonInput;

use crate::{any_factory::AnyFactory, factory::Factory, generate_fallback_provider};

pub struct Chain<P: Provider + Clone> {
    provider: P,
    factories: Vec<AnyFactory<P>>,
}
impl<P: Provider + Clone> Chain<P> {
    pub fn new(json: &ChainJsonInput, _db: &ChainsDB) {
        let urls = json.http_nodes_urls.clone();
        let provider = generate_fallback_provider(urls);
        let factories = json
            .dexes
            .iter()
            .for_each(|x| AnyFactory::new(x, provider.clone()));

        Self {
            provider,
            factories,
        }
    }
}
