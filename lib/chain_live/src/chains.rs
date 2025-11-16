use alloy_primitives::map::HashMap;
use alloy_provider::Provider;
use chain_db::chains_db::ChainsDB;
use chain_json::chains::ChainsJsonInput;

use crate::factory_context::SearchContext;

pub struct Chains<P: Provider + Clone> {
    providers: HashMap<u64, P>,
    db: ChainsDB,
    ctx: SearchContext,
}

impl<P: Provider + Clone> Default for Chains<P> {
    fn default() -> Self {
        let default_data = ChainsJsonInput::default();
        let default_db = ChainsDB::default();

        Self {
            providers: todo!(),
            db: todo!(),
            ctx: todo!(),
        }
    }
}
