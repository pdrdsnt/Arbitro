use std::sync::Arc;

use ethers::{
    abi::{parse_abi, Abi, RawLog, Token, Tokenizable},
    types::{Address, Block, Log, Transaction, H160, H256},
    utils::hex,
};

use crate::AbisData;

pub struct Decoder();

impl Decoder {
    pub fn decode_tx_static(
        abis: &AbisData, 
        tx: &Transaction
    ) -> DecodedTx {
        if let Some(to_addr) = tx.to {
            let input_bytes = &tx.input.0;

            if input_bytes.len() < 4 {
                println!("— no calldata for tx {:?}", tx.hash);
                return DecodedTx::Unknown { 
                    selector: [0u8; 4], 
                    to: to_addr 
                };
            }

            let selector: [u8; 4] = input_bytes[0..4].try_into().unwrap();

            if let Some(func) = abis
                .v2_pool
                .functions()
                .find(|f| f.short_signature() == selector)
            {
                match func.decode_input(&input_bytes[4..]) {
                    Ok(tokens) => {
                        return DecodedTx::V2 { 
                            func: func.name.clone(), 
                            tokens 
                        };
                    }
                    Err(e) => {
                        eprintln!("decode_input error: {:?}", e);
                        return DecodedTx::Unknown { selector, to: to_addr };
                    }
                }
            }

            if let Some(func) = abis
                .v3_pool
                .functions()
                .find(|f| f.short_signature() == selector)
            {
                match func.decode_input(&input_bytes[4..]) {
                    Ok(tokens) => {
                        return DecodedTx::V3 { 
                            func: func.name.clone(), 
                            tokens 
                        };
                    }
                    Err(e) => {
                        eprintln!("decode_input error: {:?}", e);
                        return DecodedTx::Unknown { selector, to: to_addr };
                    }
                }
            }

            if let Some(func) = abis
                .bep_20
                .functions()
                .find(|k| (k.name == "transfer" || k.name == "approve") && k.short_signature() == selector)
            {
                match func.decode_input(&input_bytes[4..]) {
                    Ok(tokens) => {
                        return DecodedTx::Token { 
                            func: func.name.clone(), 
                            tokens 
                        };
                    }
                    Err(e) => {
                        eprintln!("decode_input error: {:?}", e);
                        return DecodedTx::Unknown { selector, to: to_addr };
                    }
                }
            }

            return DecodedTx::Unknown { selector, to: to_addr };
        }

        // No `to` field
        DecodedTx::Unknown { selector: [0u8; 4], to: Address::zero() }
    }
}

#[derive(Debug)]
pub enum DecodedTx {
    V2 { func: String, tokens: Vec<Token> },
    V3 { func: String, tokens: Vec<Token> },
    Token { func: String, tokens: Vec<Token> },
    Unknown { selector: [u8; 4], to: Address },
}