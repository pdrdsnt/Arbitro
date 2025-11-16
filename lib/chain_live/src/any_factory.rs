use std::{cell::RefCell, str::FromStr};

use alloy_primitives::{
    Address,
    aliases::{I24, U24},
    map::HashSet,
};
use alloy_provider::Provider;
use chain_json::chain_json_model::DexJsonModel;
use sol::sol_types::{
    IUniswapV2Factory::IUniswapV2FactoryInstance, IUniswapV2Pair::IUniswapV2PairInstance,
    IUniswapV3Factory::IUniswapV3FactoryInstance,
};

use crate::{v2_factory::V2Factory, v3_factory::V3Factory, v4_factory::V4Factory};

pub enum AnyFactory<P: Provider + Clone> {
    V2(V2Factory<P>),
    V3(V3Factory<P>),
    V4(V4Factory<P>),
}

impl<P: Provider + Clone> AnyFactory<P> {
    fn new(value: DexJsonModel, provider: P) -> Option<Self> {
        match value {
            DexJsonModel::V2 {
                address,
                fee,
                stable_fee,
            } => {
                let addr = Address::from_str(&address).ok()?;
                let f = V2Factory {
                    name: "".to_string(),
                    contract: IUniswapV2FactoryInstance::new(addr, provider.clone()),
                    fee: U24::from(fee),
                };
                AnyFactory::V2(f).into()
            }
            DexJsonModel::V3 { address, fee } => {
                let addr = Address::from_str(&address).ok()?;
                let f = V3Factory::new();
                AnyFactory::V3(f).into()
            }
            DexJsonModel::V4 { address } => todo!(),
        }
    }
}
