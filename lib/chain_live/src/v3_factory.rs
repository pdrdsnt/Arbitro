use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
use alloy_sol_types::abi::Token;
use chain_db::{
    p_config::{AnyPoolConfig, V3Config},
    p_key::SledPairKey,
    p_ticks::PoolWords,
    p_tokens::Tokens,
};
use futures::future::join_all;
use sol::sol_types::{IUniswapV3Factory::IUniswapV3FactoryInstance, V3Pool::V3PoolInstance};

use crate::{
    factory::{FEES, Factory, TICK_SPACES},
    factory_context::{self, SearchContext},
    v3_pool::{V3Data, V3Pool},
};

pub struct V3Factory<P: Provider + Clone> {
    pub name: String,
    pub contract: IUniswapV3FactoryInstance<P>,
    tried: RefCell<HashSet<V3Config>>,
    pub ctx: Arc<SearchContext>,
    pub chain: u64,
    pub targets: Vec<Address>,
}

impl<P: Provider + Clone> V3Factory<P> {
    pub fn new(
        name: String,
        addr: Address,
        provider: P,
        ctx: Arc<SearchContext>,
        chain: u64,
    ) -> Self {
        Self {
            name,
            contract: IUniswapV3FactoryInstance::new(addr, provider.clone()),
            tried: RefCell::new(HashSet::new()),
            ctx,
            chain,
            targets: vec![],
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct V3Key((Address, Address, U24));

impl<P: Provider + Clone> Factory<P> for V3Pool<P> {
    fn get_space(&self) -> Vec<AnyPoolConfig> {
        let mut vec = vec![];
        let targets = self.get_targets();

        for fee in FEES {
            for spacing in TICK_SPACES {}
            for token0 in targets {}
        }
    }

    fn get_ctx(&self) -> &crate::factory_context::SearchContext {
        todo!()
    }

    fn get_targets(&self) -> Vec<SledPairKey> {
        todo!()
    }

    fn create_calls(&self) {
        let arr = self.get_space();

        for any_config in arr {
            match any_config {
                AnyPoolConfig::V2(v2_config) => {}
                AnyPoolConfig::V3(v3_config) => {}
                AnyPoolConfig::V4(v4_config) => {}
            }
        }
    }
}
