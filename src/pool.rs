use crate::{
    mult_provider::MultiProvider,
    pool_utils::{Tick, Trade},
    token::Token,
};
use axum::http::version;
use bigdecimal::{self, BigDecimal, FromPrimitive};
use clap::error;
use ethers::{
    abi::{ethabi::AbiError, Address},
    contract::{Contract, ContractError},
    core::k256::{elliptic_curve::{
        bigint,
        consts::{U16, U160, U2, U24, U25, U8},
    }, pkcs8::der::oid::Arc},
    providers::{Http, Provider},
    types::{Res, H160, U256},
};
use futures::future::join_all;
use num_traits::{float, Float};
use std::ops::{BitAnd, Mul};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ptr::read,
    str::FromStr,
};
use thiserror::Error;
pub trait Pool {
    async fn update(&mut self) -> Result<(), PoolUpdateError>;
    fn trade(&self, amount_in: U256, from: bool) -> Option<Trade>;
}

// 1. Declare a unified error type:
#[derive(Debug, Error)]
pub enum PoolUpdateError {
    #[error("ABI error: {0}")]
    Abi(#[from] ethers::contract::AbiError),

    #[error("Contract call error: {0}")]
    Rpc(#[from] ContractError<Provider<MultiProvider>>),
}
