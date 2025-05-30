use ethers::contract::ContractError;
use ethers_providers::Provider;
use thiserror::Error;

use crate::mult_provider::MultiProvider;

// 1. Declare a unified error type:
#[derive(Debug, Error)]
pub enum PoolUpdateError {
    #[error("ABI error: {0}")]
    Abi(#[from] ethers::contract::AbiError),

    #[error("Contract call error: {0}")]
    Rpc(#[from] ContractError<Provider<MultiProvider>>),
    
    #[error("{0}")]
    Custom(String),
}

// manually impl From<String>
impl From<String> for PoolUpdateError {
    fn from(s: String) -> Self {
        PoolUpdateError::Custom(s)
    }
}