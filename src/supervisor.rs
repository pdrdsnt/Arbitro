use std::{str::FromStr, string, sync::Arc};

use anyhow::Chain;
use ethers::{
    abi::{Address, Log},
    contract::EthEvent,
    types::{BlockNumber, Filter, Transaction, ValueOrArray, H160, H256},
    utils::keccak256,
};
use ethers_providers::{Provider, Ws};
use futures::io::Seek;
use tokio::sync::Mutex;

use crate::{
    arbitro::Arbitro,
    block_decoder::Decoder,
    blockchain_db::{DexModel, TokenModel},
    chain_src::ChainSrc,
    chain_svc::ChainDataService,
    factory::AnyFactory,
    mapped_vec::MappedVec,
    mem_pool,
    mult_provider::MultiProvider,
    pool_action::PoolAction,
    simulacrum::Simulacrum,
    v_pool_sim::AnyPoolSim,
    AbisData,
};

pub struct Supervisor {
    chain_data: ChainData,
    chain_settings: ChainSettings,
    block_service: ChainDataService,
    simulacrum: Simulacrum<Arbitro, H160, PoolAction>,
    chain_src: ChainSrc,
}

impl Supervisor {
    pub async fn new(chain_data: ChainData, chain_settings: ChainSettings) -> Self {
        let _chain_data = chain_data.clone();
        let arc_ws_provider = Arc::new(_chain_data.ws_providers);
        let arc_http_provider = Arc::new(_chain_data.http_providers);
        let arc_abi_provider = Arc::new(_chain_data.abis);

        let svc = ChainDataService { ws_providers: arc_ws_provider };

        let src = ChainSrc::new(
            arc_abi_provider,
            arc_http_provider,
            &chain_settings.tokens,
            &chain_settings.factories, // chain_data.tokens_list,
        )
        .await;

        let abt = Arbitro::new(MappedVec::from_array(src.create_sim().await));
        let sim: Simulacrum<Arbitro, H160, PoolAction> = Simulacrum::new(abt);
        Self { chain_data, chain_settings, block_service: svc, simulacrum: sim, chain_src: src }
    }

    async fn start(mut self) {
        // Convert token addresses to H160
        let addresses: Vec<H160> = self
            .chain_settings
            .tokens
            .iter()
            .map(|t| H160::from_str(&t.address))
            .collect::<Result<Vec<_>, _>>()
            .expect("Invalid token address format");

        // Create proper filter
        let filter =
            Filter::new().from_block(BlockNumber::Latest).address(ValueOrArray::Array(addresses));

        // Spawn the log subscriber
        let mut log_rx = self.block_service.spawn_log_subscriber(filter);
        let mut mempool_rx = self.block_service.spawn_mempool_subscriber();

        let shared_arbitro = Arc::new(Mutex::new(self.simulacrum));
        let _shared_arbitro = shared_arbitro.clone();
        tokio::spawn(async move {
            while let Some(mem_pool) = mempool_rx.recv().await {
                let d = Decoder::decode_tx_static(&self.chain_data.abis, &mem_pool);
                 let maybe_action: Option<PoolAction> = match decoded {
                    DecodedTx::V2 { func, tokens } => {
                        // V2 “methods” might be: "swap", "mint", "burn", etc.
                        match func.as_str() {
                            "swap" => {
                                //
                                //   V2 swap ABI is something like:
                                //   function swap(
                                //       address sender,
                                //       uint256 amount0In,
                                //       uint256 amount1In,
                                //       uint256 amount0Out,
                                //       uint256 amount1Out,
                                //       address to
                                //   )
                                //
                                //   So `tokens` Vec<Token> should be:
                                //   [ Token::Address(sender),
                                //     Token::Uint(amount0In),
                                //     Token::Uint(amount1In),
                                //     Token::Uint(amount0Out),
                                //     Token::Uint(amount1Out),
                                //     Token::Address(to) ]
                                //
                                if let [
                                    Token::Address(sender),
                                    Token::Uint(amount0_in),
                                    Token::Uint(amount1_in),
                                    Token::Uint(amount0_out),
                                    Token::Uint(amount1_out),
                                    Token::Address(to),
                                ] = &tokens[..]
                                {
                                    Some(PoolAction::SwapV2 {
                                        sender: *sender,
                                        amount0_in: *amount0_in,
                                        amount1_in: *amount1_in,
                                        amount0_out: *amount0_out,
                                        amount1_out: *amount1_out,
                                        to: *to,
                                    })
                                } else {
                                    // If the shape is not exactly what we expect, drop it.
                                    None
                                }
                            }

                            "mint" => {
                                //
                                //   V2 mint ABI is something like:
                                //   function mint(address sender, uint256 amount0, uint256 amount1)
                                //
                                if let [
                                    Token::Address(sender),
                                    Token::Uint(amount0),
                                    Token::Uint(amount1),
                                ] = &tokens[..]
                                {
                                    Some(PoolAction::MintV2 {
                                        sender: *sender,
                                        amount0: *amount0,
                                        amount1: *amount1,
                                    })
                                } else {
                                    None
                                }
                            }

                            "burn" => {
                                //
                                //   V2 burn ABI is something like:
                                //   function burn(
                                //       address sender,
                                //       uint256 amount0,
                                //       uint256 amount1,
                                //       address to
                                //   )
                                //
                                if let [
                                    Token::Address(sender),
                                    Token::Uint(amount0),
                                    Token::Uint(amount1),
                                    Token::Address(to),
                                ] = &tokens[..]
                                {
                                    Some(PoolAction::BurnV2 {
                                        sender: *sender,
                                        amount0: *amount0,
                                        amount1: *amount1,
                                        to: *to,
                                    })
                                } else {
                                    None
                                }
                            }

                            _ => {
                                // Not a V2 pool‐method we care about.
                                None
                            }
                        }
                    }

                    DecodedTx::V3 { func, tokens } => {
                        // V3 “methods” might be: "swap", "mint", "burn"
                        // But the V3 swap ABI signature usually is:
                        //   function swap(
                        //     address sender,
                        //     address recipient,
                        //     int256 amount0,
                        //     int256 amount1,
                        //     uint160 sqrtPriceX96,
                        //     uint128 liquidity,
                        //     int24 tick
                        //   );
                        match func.as_str() {
                            "swap" => {
                                if let [
                                    Token::Address(sender),
                                    Token::Address(recipient),
                                    Token::Int(amount0),
                                    Token::Int(amount1),
                                    Token::Uint(sqrt_price_x96),
                                    Token::Uint(liquidity),
                                    Token::Int(tick_raw),
                                ] = &tokens[..]
                                {
                                    // `tick_raw` is a signed integer, but ethers::abi encodes int24 as `Token::Int(I256)`.
                                    // We need to downcast that I256 to i32:
                                    let tick_i32: i32 = {
                                        // This unwrap is safe if the value fits in i32.
                                        let full = amount0.clone(); // just to satisfy types
                                        // ... but wait: we actually matched `Token::Int(tick_raw)`, so:
                                        let i256_tick = if let Token::Int(inner) = tick_raw { *inner } else { I256::zero() };
                                        i256_tick.as_i64() as i32
                                    };

                                    Some(PoolAction::SwapV3 {
                                        sender: *sender,
                                        recipient: *recipient,
                                        amount0: *amount0,
                                        amount1: *amount1,
                                        sqrt_price_x96: *sqrt_price_x96,
                                        liquidity: *liquidity,
                                        tick: tick_i32,
                                    })
                                } else {
                                    None
                                }
                            }

                            "mint" => {
                                // V3 mint ABI:
                                //   function mint(
                                //     address sender,
                                //     address owner,
                                //     int24 tickLower,
                                //     int24 tickUpper,
                                //     uint128 amount,
                                //     uint256 amount0,
                                //     uint256 amount1
                                //   );
                                if let [
                                    Token::Address(sender),
                                    Token::Address(owner),
                                    Token::Int(tick_lower_raw),
                                    Token::Int(tick_upper_raw),
                                    Token::Uint(amount),
                                    Token::Uint(amount0),
                                    Token::Uint(amount1),
                                ] = &tokens[..]
                                {
                                    let tick_lower_i32 = if let Token::Int(inner) = tick_lower_raw {
                                        inner.as_i64() as i32
                                    } else {
                                        0
                                    };
                                    let tick_upper_i32 = if let Token::Int(inner) = tick_upper_raw {
                                        inner.as_i64() as i32
                                    } else {
                                        0
                                    };

                                    Some(PoolAction::MintV3 {
                                        sender: *sender,
                                        owner: *owner,
                                        tick_lower: tick_lower_i32,
                                        tick_upper: tick_upper_i32,
                                        amount: *amount,
                                        amount0: *amount0,
                                        amount1: *amount1,
                                    })
                                } else {
                                    None
                                }
                            }

                            "burn" => {
                                // V3 burn ABI:
                                //   function burn(
                                //     address owner,
                                //     int24 tickLower,
                                //     int24 tickUpper,
                                //     uint128 amount,
                                //     uint256 amount0,
                                //     uint256 amount1
                                //   );
                                if let [
                                    Token::Address(owner),
                                    Token::Int(tick_lower_raw),
                                    Token::Int(tick_upper_raw),
                                    Token::Uint(amount),
                                    Token::Uint(amount0),
                                    Token::Uint(amount1),
                                ] = &tokens[..]
                                {
                                    let tick_lower_i32 = if let Token::Int(inner) = tick_lower_raw {
                                        inner.as_i64() as i32
                                    } else {
                                        0
                                    };
                                    let tick_upper_i32 = if let Token::Int(inner) = tick_upper_raw {
                                        inner.as_i64() as i32
                                    } else {
                                        0
                                    };

                                    Some(PoolAction::BurnV3 {
                                        owner: *owner,
                                        tick_lower: tick_lower_i32,
                                        tick_upper: tick_upper_i32,
                                        amount: *amount,
                                        amount0: *amount0,
                                        amount1: *amount1,
                                    })
                                } else {
                                    None
                                }
                            }

                            _ => None,
                        }
                    }
            }
        });

        while let Some(log) = log_rx.recv().await {
            if let Some((action, addr)) = PoolAction::parse_pool_action(&log) {
                shared_arbitro.lock().await.origin_mut().update_state(&addr, action);
            }
        }
    }

    pub fn process_mempool() {
        // duplicate arbitro and update state of one pool in the new arbitro
        // maybe a new struc that manages multiple arbitros and the specific modifications
        // mempool operations can fail so multiple possibilities
        // pools - modifications -
        // H160  - [{block, provider, trade}]
    }
}

#[derive(Clone)]
pub struct ChainData {
    id: u32,
    name: String,
    abis: AbisData,
    ws_providers: Vec<Provider<Ws>>,
    http_providers: Provider<MultiProvider>,
}

#[derive(Clone)]
pub struct ChainSettings {
    tokens: Vec<TokenModel>,
    factories: Vec<DexModel>,
}
