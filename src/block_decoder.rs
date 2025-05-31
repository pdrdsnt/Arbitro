use std::sync::Arc;

use ethers::{
    abi::{parse_abi, Abi, RawLog, Token, Tokenizable},
    types::{Address, Block, Log, Transaction, H160, H256},
    utils::hex,
};

use crate::AbisData;

pub struct Decoder();

impl Decoder {
    /// Attempts to decode *only* the `tx.input` portion of a `Transaction`.
    /// Returns a light‐weight `DecodedTx` enum holding the function‐name + tokens.
    pub fn decode_tx_static(abis: &AbisData, tx: &Transaction) -> DecodedTx {
        // If `to` is None, we cannot decode against any known pool or token ABI.
        let to_addr = match tx.to {
            Some(addr) => addr,
            None => {
                eprintln!("decode_tx_static: missing `to` for tx {:?}", tx.hash);
                return DecodedTx::Unknown {
                    selector: [0u8; 4],
                    to: Address::zero(),
                };
            }
        };

        let input_bytes = &tx.input.0;
        if input_bytes.len() < 4 {
            // No selector at all—no calldata to decode.
            println!("— no calldata for tx {:?}", tx.hash);
            return DecodedTx::Unknown {
                selector: [0u8; 4],
                to: to_addr,
            };
        }

        // First 4 bytes = “selector”
        let selector: [u8; 4] = input_bytes[0..4]
            .try_into()
            .expect("slice of length ≥ 4; qed");

        // 1) Try to match against the V2‐pool ABI
        if let Some(func) = abis.v2_pool.functions().find(|f| f.short_signature() == selector) {
            match func.decode_input(&input_bytes[4..]) {
                Ok(tokens) => {
                    return DecodedTx::V2 {
                        func: func.name.clone(),
                        tokens,
                    };
                }
                Err(err) => {
                    eprintln!("decode_input (V2) error: {:?}", err);
                    return DecodedTx::Unknown { selector, to: to_addr };
                }
            }
        }

        // 2) Try to match against the V3‐pool ABI
        if let Some(func) = abis.v3_pool.functions().find(|f| f.short_signature() == selector) {
            match func.decode_input(&input_bytes[4..]) {
                Ok(tokens) => {
                    return DecodedTx::V3 {
                        func: func.name.clone(),
                        tokens,
                    };
                }
                Err(err) => {
                    eprintln!("decode_input (V3) error: {:?}", err);
                    return DecodedTx::Unknown { selector, to: to_addr };
                }
            }
        }

        // 3) Try to match BEP‐20 “transfer” or “approve”
        if let Some(func) = abis.bep_20.functions().find(|k| {
            (k.name == "transfer" || k.name == "approve") && k.short_signature() == selector
        }) {
            match func.decode_input(&input_bytes[4..]) {
                Ok(tokens) => {
                    return DecodedTx::Token {
                        func: func.name.clone(),
                        tokens,
                    };
                }
                Err(err) => {
                    eprintln!("decode_input (BEP‐20) error: {:?}", err);
                    return DecodedTx::Unknown { selector, to: to_addr };
                }
            }
        }

        // Nothing matched.
        DecodedTx::Unknown { selector, to: to_addr }
    }

    /// Given a `DecodedTx::V2 { .. }` or `DecodedTx::V3 { .. }`, attempt to
    /// convert it into a `PoolAction` (SwapV2, MintV2, BurnV3, etc.).  
    /// Returns `None` if the function‐name + tokens don’t match the expected shape.
    fn decode_tx_to_action(decoded: DecodedTx) -> Option<PoolAction> {
        match decoded {
            DecodedTx::V2 { func, tokens } => {
                match func.as_str() {
                    // V2‐pool “swap”:
                    //   function swap(
                    //     address sender,
                    //     uint256 amount0In,
                    //     uint256 amount1In,
                    //     uint256 amount0Out,
                    //     uint256 amount1Out,
                    //     address to
                    //   )
                    "swap" => {
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
                            None
                        }
                    }

                    // V2‐pool “mint”:
                    //   function mint(address sender, uint256 amount0, uint256 amount1)
                    "mint" => {
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

                    // V2‐pool “burn”:
                    //   function burn(
                    //     address sender,
                    //     uint256 amount0,
                    //     uint256 amount1,
                    //     address to
                    //   )
                    "burn" => {
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

                    _ => None,
                }
            }

            DecodedTx::V3 { func, tokens } => {
                match func.as_str() {
                    // V3‐pool “swap”:
                    //   function swap(
                    //     address sender,
                    //     address recipient,
                    //     int256 amount0,
                    //     int256 amount1,
                    //     uint160 sqrtPriceX96,
                    //     uint128 liquidity,
                    //     int24 tick
                    //   );
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
                            // Directly downcast the I256 → i32.
                            let tick_i32 = tick_raw.as_i64() as i32;
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

                    // V3‐pool “mint”:
                    //   function mint(
                    //     address sender,
                    //     address owner,
                    //     int24 tickLower,
                    //     int24 tickUpper,
                    //     uint128 amount,
                    //     uint256 amount0,
                    //     uint256 amount1
                    //   );
                    "mint" => {
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
                            let tick_lower_i32 = tick_lower_raw.as_i64() as i32;
                            let tick_upper_i32 = tick_upper_raw.as_i64() as i32;
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

                    // V3‐pool “burn”:
                    //   function burn(
                    //     address owner,
                    //     int24 tickLower,
                    //     int24 tickUpper,
                    //     uint128 amount,
                    //     uint256 amount0,
                    //     uint256 amount1
                    //   );
                    "burn" => {
                        if let [
                            Token::Address(owner),
                            Token::Int(tick_lower_raw),
                            Token::Int(tick_upper_raw),
                            Token::Uint(amount),
                            Token::Uint(amount0),
                            Token::Uint(amount1),
                        ] = &tokens[..]
                        {
                            let tick_lower_i32 = tick_lower_raw.as_i64() as i32;
                            let tick_upper_i32 = tick_upper_raw.as_i64() as i32;
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

            // We don’t produce a `PoolAction` for a “Token { … }” or for `Unknown { … }`
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum DecodedTx {
    V2 { func: String, tokens: Vec<Token> },
    V3 { func: String, tokens: Vec<Token> },
    Token { func: String, tokens: Vec<Token> },
    Unknown { selector: [u8; 4], to: Address },
}
