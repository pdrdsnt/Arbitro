use alloy_provider::Provider;
use chain_db::chains_db::ChainsDB;
use chain_json::chain::ChainJsonInput;

use crate::{
    any_factory::AnyFactory, chains::Chains, factory::Factory, generate_fallback_provider,
    search_context::SearchContext, token::Token,
};

pub struct Chain<'a, P: Provider + Clone> {
    provider: P,
    factories: Vec<AnyFactory<'a, P>>,
    tokens: Vec<Token<P>>,
    ctx: &'a SearchContext,
}
impl<'a, P: Provider + Clone> Chain<'a, P> {
    pub fn new(json: &ChainJsonInput, provider: P, ctx: &'a SearchContext, chain_id: u64) -> Self {
        let urls = json.http_nodes_urls.clone();
        let factories = json
            .dexes
            .iter()
            .filter_map(|x| AnyFactory::from_json_model(x, provider.clone(), ctx, chain_id))
            .collect();

        Self {
            provider,
            factories,
            ctx,
        }
    }
}
