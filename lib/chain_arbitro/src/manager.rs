use std::collections::BTreeMap;

use alloy::providers::Provider;
use chains::chains::Chains;

use crate::chain_arbitro::ChainArbitro;

pub struct ArbitroManager<P: Provider + Clone> {
    arbitros: BTreeMap<u64, ChainArbitro<P>>,
}

impl<P: Provider + Clone> ArbitroManager<P> {
    pub fn new(chains: Chains, provider: P) -> Self {
        let map = BTreeMap::new();
        for (id, chain) in chains.chains {
            let new = ChainArbitro::from_chain(&chain, provider.clone());
        }
        Self { arbitros: map }
    }
}
