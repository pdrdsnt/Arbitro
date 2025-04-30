#![allow(warnings)]
mod factory;
mod anvil_test;
mod arbitro;
mod blockchain_db;
mod dex;
mod mem_pool;
mod mult_provider;
mod pair;
mod pool;
mod pool_utils;
mod token;
mod seeker;
mod v2_pool;
mod v3_pool;

use abi::{Abi, Address};
use arbitro::Arbitro;
use axum::extract::path;
use blockchain_db::{ABIModel, BlockChainsModel, DexModel, TokenModel};
use dex::{AnyDex, Dex};
use ethers::abi::{encode, Detokenize, Tokenizable};
use ethers::utils::hex;
use ethers::{contract::*, core::k256::elliptic_curve::consts::U2, prelude::*};
use mult_provider::MultiProvider;
use pair::Pair;
use pool::Pool;
use pool_utils::{AbisData, Trade};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env::home_dir;
use std::fs;
use std::process::Command;
use std::time::Duration;
use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};
use tokio::{sync::RwLock, time::error::Error};

#[tokio::main]
async fn main() -> Result<(), ethers::providers::ProviderError> {
    let mut urls = vec![
        "https://bsc-rpc.publicnode.com".to_string(),
        "https://bsc-dataseed.binance.org/".to_string(),
        // Additional endpoints
        "https://bsc-dataseed.bnbchain.org".to_string(),
        "https://bsc-dataseed.nariox.org".to_string(),
        "https://bsc-dataseed.defibit.io".to_string(),
        "https://bsc-mainnet.public.blastapi.io".to_string(),
        "https://go.getblock.io/84ccc9310a1f40ce9efa177d3e949b3c".to_string(),
    ];

    // Wrap into ethers Provider
    let mut mult_provider: MultiProvider =
        mult_provider::MultiProvider::new(urls, Duration::from_secs(5), 3);

    let provider = Provider::<MultiProvider>::new(mult_provider);
    let chains_data = BlockChainsModel::new("src/chainsData.json").unwrap();
    let _dexes = &chains_data.chains[0].dexes;
    let _tokens = &chains_data.chains[0].tokens;

    let abis = load_abis(&chains_data.chains[0].abis);

    // 1. Get basic stats
    let block: U64 = provider.get_block_number().await.unwrap();
    let v: Value = provider
        .request("txpool_content", Vec::<Value>::new())
        .await
        .unwrap();
    println!("txpool_status: {:#}", block);

    let abis_arc = Arc::new(abis);
    let mut path = Vec::<Trade>::new();
    let arc_provider = Arc::new(provider);
    let mut arbitro = Arbitro::new(_dexes, _tokens, arc_provider.clone(), abis_arc);
    arbitro.create_pools().await;

    // 2. Get full contents
    let content: Vec<TransactionReceipt> = arc_provider.get_block_receipts(block).await.unwrap();

    let pool_addrs: HashSet<H160> = arbitro.pools_lookup.iter().map(|(pool, _)| *pool).collect();
    inspect_block(&*arc_provider, block, &pool_addrs);

    let last_token_address = H160::from_str(_tokens.last().unwrap().address.as_str()).unwrap();
    let amount_in = U256::exp10(18);
    let tokens = arbitro.tokens.clone();
    let mut _block = U64::from(0);
    loop {
        let block = arc_provider.get_block_number().await?;
        if block == _block {
            continue;
        }
        _block = block;

        let r = find_first_arbitrage_path(&mut arbitro, &tokens, amount_in).await;
        inspect_block(&arc_provider, block, &pool_addrs).await?;
        arbitro.update_pools().await;

        for _token in tokens.iter() {
            let token = _token.read().await;
           
            let _paths = arbitro.arbitrage(&token.address, amount_in).await;
            if _paths.is_empty() {
                println!("No arbitrage paths found for token {}", token.name);
                continue;
            } else {
                path = _paths[0].clone();
            }
        }
        println!("=============================");
        tokio::time::sleep(Duration::from_secs(10)).await;
        
    }

    Ok(())
}

use ethers::abi::{InvalidOutputType, Token};
pub async fn find_first_arbitrage_path(
    arbitro: &mut Arbitro,
    tokens: &[Arc<RwLock<token::Token>>],
    amount_in: U256,
) -> Option<Vec<Trade>> {
    for token_lock in tokens {
        let token = token_lock.read().await;
        let paths = arbitro.arbitrage(&token.address, amount_in).await;
        if paths.is_empty() {
            println!("No arbitrage paths found for token {}", token.name);
            continue;
        }
        // got at least one, return its first path
        return Some(paths[0].clone());
    }
    // none of the tokens yielded any paths
    None
}
async fn inspect_block(
    arc_provider: &Provider<MultiProvider>,
    block: U64,
    // your lookup of pool addresses
    pool_addrs: &HashSet<H160>,
) -> Result<(), ethers::providers::ProviderError> {
    // thresholds for “big”
    // here: > 100 ETH (100 * 10^18 wei)
    let VALUE_THRESHOLD: U256 = U256::exp10(20);
    // here: nonce > 1000
    const NONCE_THRESHOLD: u64 = 100;

    // 1. Get all receipts
    let receipts: Vec<TransactionReceipt> = arc_provider.get_block_receipts(block).await?;

    for receipt in receipts {
        // 2. Quick pool‐filter on the `to` field of the receipt
        if let Some(to_addr) = receipt.to {
            if pool_addrs.contains(&to_addr) {
                println!("▷ Receipt TO one of our pools: {:?}", to_addr);
            }
        }

        // 3. Fetch the full Transaction so we can see `value` & `nonce`
        let tx_hash: H256 = receipt.transaction_hash;
        if let Some(tx) = arc_provider.get_transaction(tx_hash).await? {
            // 4. Check for a “big value” tx
            if tx.value > VALUE_THRESHOLD {
                println!("🔶 High-value tx (> {} wei):\n{:#?}", VALUE_THRESHOLD, tx);
            }

        }
    }

    Ok(())
}
// Function to load ABIs
fn load_abis(abis: &ABIModel) -> AbisData {
    let v2_factory_abi_json = serde_json::to_string(&abis.v2_factory_abi).unwrap();
    let v2_factory_abi_ethers = serde_json::from_str::<Abi>(&v2_factory_abi_json).unwrap();

    let bep_20_abi_json = serde_json::to_string(&abis.bep_20_abi).unwrap();
    let bep_20_abi_ethers: Abi = serde_json::from_str::<Abi>(&bep_20_abi_json).unwrap();

    let v3_factory_abi_json = serde_json::to_string(&abis.v3_factory_abi).unwrap();
    let v3_factory_abi_ethers = serde_json::from_str::<Abi>(&v3_factory_abi_json).unwrap();

    let v2_pool_abi_json = serde_json::to_string(&abis.v2_pool_abi).unwrap();
    let v2_pool_abi_ethers = serde_json::from_str::<Abi>(&v2_pool_abi_json).unwrap();

    let v3_pool_abi_json = serde_json::to_string(&abis.v3_pool_abi).unwrap();
    let v3_pool_abi_ethers = serde_json::from_str::<Abi>(&v3_pool_abi_json).unwrap();

    AbisData {
        v2_factory: v2_factory_abi_ethers,
        v2_pool: v2_pool_abi_ethers,
        v3_factory: v3_factory_abi_ethers,
        v3_pool: v3_pool_abi_ethers,
        bep_20: bep_20_abi_ethers,
    }
}
#[derive(Clone, Debug)]
struct Currency {
    // Assuming Currency has its own fields, adjust accordingly
    is_token: bool,
    value: Address,
}

impl Tokenizable for Currency {
    fn into_token(self) -> Token {
        // Adjust based on actual Currency structure
        Token::Tuple(vec![Token::Bool(self.is_token), Token::Address(self.value)])
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        let tuple = token
            .into_tuple()
            .ok_or(InvalidOutputType("Expected tuple for Currency".to_string()))?;
        let mut elements = tuple.into_iter();

        Ok(Currency {
            is_token: elements
                .next()
                .ok_or(InvalidOutputType("Missing is_token".to_string()))?
                .into_bool()
                .ok_or(InvalidOutputType("Invalid is_token type".to_string()))?,
            value: elements
                .next()
                .ok_or(InvalidOutputType("Missing value".to_string()))?
                .into_address()
                .ok_or(InvalidOutputType("Invalid value type".to_string()))?,
        })
    }
}

#[derive(Clone, Debug)]
struct PoolKey {
    currency0: Currency,
    currency1: Currency,
    fee: u32,         // uint24
    tickSpacing: i32, // int24
    hooks: Address,   // IHooks is represented as Address
}
impl Tokenizable for PoolKey {
    fn into_token(self) -> Token {
        Token::Tuple(vec![
            self.currency0.into_token(),
            self.currency1.into_token(),
            Token::Uint(U256::from(self.fee)),
            Token::Int(int_to_uint256(self.tickSpacing)),
            Token::Address(self.hooks),
        ])
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        let tuple = token
            .into_tuple()
            .ok_or(InvalidOutputType("Expected tuple for PoolKey".to_string()))?;
        let mut elems = tuple.into_iter();

        let c0 = Currency::from_token(
            elems
                .next()
                .ok_or(InvalidOutputType("Missing currency0".into()))?,
        )?;
        let c1 = Currency::from_token(
            elems
                .next()
                .ok_or(InvalidOutputType("Missing currency1".into()))?,
        )?;

        // fee
        let raw_fee = elems
            .next()
            .and_then(|t| t.into_uint())
            .ok_or(InvalidOutputType("Missing or invalid `fee`".into()))?
            .as_u32();
        let fee = require_range_u32(raw_fee, 0xFF_FFFF, "fee")?;

        // tickSpacing
        let raw_tick = elems
            .next()
            .and_then(|t| t.into_int())
            .ok_or(InvalidOutputType("Missing or invalid `tickSpacing`".into()))?
            .as_u64() as i32;
        let tickSpacing = require_range_i32(raw_tick, -0x800000, 0x7F_FFFF, "tickSpacing")?;

        let hooks = elems
            .next()
            .and_then(|t| t.into_address())
            .ok_or(InvalidOutputType("Missing or invalid `hooks`".into()))?;

        Ok(PoolKey {
            currency0: c0,
            currency1: c1,
            fee,
            tickSpacing,
            hooks,
        })
    }
}
#[derive(Clone, Debug)]
struct Hooks(Address);

impl From<Address> for Hooks {
    fn from(addr: Address) -> Self {
        Hooks(addr)
    }
}

// Implement any needed trait conversions
impl Tokenizable for Hooks {
    fn into_token(self) -> Token {
        Token::Address(self.0)
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        Ok(Hooks(
            token
                .into_address()
                .ok_or(InvalidOutputType("".to_string()))?,
        ))
    }
}

#[derive(serde::Serialize)]
enum PoolType {
    UNISWAP_V2,
    PANCAKE_V2,
    UNISWAP_V3,
    PANCAKE_V3,
    UNISWAP_V4,
    PANCAKE_V4,
}

impl Tokenizable for PoolType {
    fn into_token(self) -> Token {
        match self {
            PoolType::UNISWAP_V2 => Token::Uint(U256::from(0)),
            PoolType::PANCAKE_V2 => Token::Uint(U256::from(1)),
            PoolType::UNISWAP_V3 => Token::Uint(U256::from(2)),
            PoolType::PANCAKE_V3 => Token::Uint(U256::from(3)),
            PoolType::UNISWAP_V4 => Token::Uint(U256::from(4)),
            PoolType::PANCAKE_V4 => Token::Uint(U256::from(5)),
        }
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        let value = token
            .into_uint()
            .ok_or(InvalidOutputType("Expected uint for PoolType".to_string()))?;
        match value.as_u32() {
            0 => Ok(PoolType::UNISWAP_V2),
            1 => Ok(PoolType::PANCAKE_V2),
            2 => Ok(PoolType::UNISWAP_V3),
            3 => Ok(PoolType::PANCAKE_V3),
            4 => Ok(PoolType::UNISWAP_V4),
            5 => Ok(PoolType::PANCAKE_V4),
            _ => Err(InvalidOutputType("Invalid PoolType".to_string())),
        }
    }
}

struct PoolSpec {
    addr: Address,
    key: PoolKey,
    pool_typ: PoolType,
    from0: bool,
}

impl Tokenizable for PoolSpec {
    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::Address(self.addr),
            self.key.into_token(),
            self.pool_typ.into_token(),
            Token::Bool(self.from0),
        ])
    }

    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        let tuple = token
            .into_tuple()
            .ok_or(InvalidOutputType("Expected tuple for PoolSpec".to_string()))?;
        let mut elements = tuple.into_iter();

        Ok(PoolSpec {
            addr: elements
                .next()
                .ok_or(InvalidOutputType("Missing addr".to_string()))?
                .into_address()
                .ok_or(InvalidOutputType("Invalid addr type".to_string()))?,
            key: PoolKey::from_token(
                elements
                    .next()
                    .ok_or(InvalidOutputType("Missing key".to_string()))?,
            )?,
            pool_typ: PoolType::from_token(
                elements
                    .next()
                    .ok_or(InvalidOutputType("Missing pool_typ".to_string()))?,
            )?,
            from0: elements
                .next()
                .ok_or(InvalidOutputType("Missing from0".to_string()))?
                .into_bool()
                .ok_or(InvalidOutputType("Invalid from0 type".to_string()))?,
        })
    }
}

// helper:
fn require_range_u32(x: u32, max: u32, name: &str) -> Result<u32, InvalidOutputType> {
    if x <= max {
        Ok(x)
    } else {
        Err(InvalidOutputType(format!("{} out of uint24 range", name)))
    }
}

fn require_range_i32(x: i32, lo: i32, hi: i32, name: &str) -> Result<i32, InvalidOutputType> {
    if (lo..=hi).contains(&x) {
        Ok(x)
    } else {
        Err(InvalidOutputType(format!("{} out of int24 range", name)))
    }
}

fn int_to_uint256(x: i32) -> U256 {
    if x >= 0 {
        U256::from(x as u64)
    } else {
        // two’s‑complement in 256‑bit space:
        let mag = U256::from((-x) as u64);
        (!mag).overflowing_add(U256::one()).0
    }
}

async fn test_round_robin_fallback() {
    // 1) First URL is invalid → will error immediately
    // 2) Second URL is a known-working public BSC RPC endpoint
    let urls = vec![
        "http://127.0.0.1:12345".to_string(),         // no server here
        "https://bsc-rpc.publicnode.com".to_string(), // should succeed
    ];

    // zero backoff so we don’t wait between retries
    let mp = MultiProvider::new(urls, Duration::from_secs(0),2);
    let provider = Provider::new(mp);

    // Use a standard RPC method with no params:
    // “net_version” returns the chain ID as a string
    let chain_id: String = provider
        .request("net_version", Vec::<Value>::new())
        .await
        .expect("fallback failed to reach working node");

    // On BSC mainnet you should get “56”
    assert_eq!(chain_id, "56", "expected BSC chain ID 56");
}
