use alloy_primitives::{Address, aliases::U24};

use alloy_provider::Provider;
use sol::sol_types::IUniswapV2Factory::IUniswapV2FactoryInstance;

use crate::v2_pool::V2Data;

#[derive(Debug)]
pub struct V2Factory<P: Provider> {
    pub name: String,
    pub contract: IUniswapV2FactoryInstance<P>,
    pub fee: U24,
}

impl<P: Provider + Clone> V2Factory<P> {
    pub async fn search_pool(&self, a: Address, b: Address) -> Option<(Address, V2Data)> {
        if let Ok(result) = self.contract.getPair(a, b).call().await {
            if result == Address::ZERO {
                return None;
            }

            let p = V2Data {
                name: Some(self.name.clone()),
                reserves: None,
                fee: Some(self.fee),
                token0: Some(a),
                token1: Some(b),
            };

            return Some((result, p));
        }

        None
    }
}
