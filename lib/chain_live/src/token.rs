use std::str::FromStr;

use alloy::providers::Provider;
use chains::chain_json_model::TokenJsonModel;
use sol::sol_types::IERC20::IERC20Instance;

pub struct Token<P: Provider> {
    pub contract: IERC20Instance<P>,
    pub decimals: u8,
    pub symbol: String,
}

impl<P: Provider> Token<P> {
    pub fn from_token_model(model: &TokenJsonModel, provider: P) -> Self {
        let contract = IERC20Instance::new(
            alloy::primitives::Address::from_str(&model.address).unwrap(),
            provider,
        );
        let decimals = model.decimals;
        let symbol = model.symbol.clone();

        Self {
            contract,
            decimals,
            symbol,
        }
    }
}
