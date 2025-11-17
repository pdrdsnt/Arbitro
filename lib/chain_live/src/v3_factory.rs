use std::{cell::RefCell, collections::HashSet, sync::Arc};

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
use chain_db::{
    p_config::{AnyPoolConfig, V3Config},
    p_key::Pair,
};
use sol::sol_types::IUniswapV3Factory::IUniswapV3FactoryInstance;

use crate::{
    chains::Chains,
    factory::{FEES, Factory, TICK_SPACES},
    search_context::SearchContext,
};

pub struct V3Factory<'a, P: Provider + Clone> {
    pub contract: IUniswapV3FactoryInstance<P>,
    pub ctx: &'a SearchContext,
    pub chain_id: u64,
}

impl<'a, P: Provider + Clone> V3Factory<'a, P> {
    pub fn new(addr: Address, provider: P, ctx: &'a SearchContext, chain_id: u64) -> Self {
        Self {
            contract: IUniswapV3FactoryInstance::new(addr, provider.clone()),
            ctx,
            chain_id,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct V3Key((Address, Address, U24));

impl<'a, P: Provider + Clone> Factory<P> for V3Factory<'a, P> {
    fn get_space(&self) -> Vec<AnyPoolConfig> {
        let mut vec = vec![];

        for (i, _fee) in FEES.iter().enumerate() {
            let Ok(fee) = U24::try_from(*_fee) else {
                continue;
            };
            let Some(_tick_spacing) = TICK_SPACES.get(i) else {
                continue;
            };
            let Ok(tick_spacing) = I24::try_from(*_tick_spacing) else {
                continue;
            };
        }
        vec
    }

    fn get_ctx(&self) -> &SearchContext {
        self.ctx
    }
}
