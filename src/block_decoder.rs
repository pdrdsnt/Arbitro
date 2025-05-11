use ethers::abi::{Abi, parse_abi, RawLog, Tokenizable};
use ethers::types::{Block, Log, Transaction, H256, H160};
use ethers::utils::hex;
use std::sync::Arc;

use crate::AbisData;

pub struct BlockDecoder {
    abis: Arc<AbisData>,
}

impl BlockDecoder {
    pub fn new(abis: Arc<AbisData>) -> Self {
        Self { abis }
    }

    /// Decode a raw block header (no ABIs needed here, just print number)
    pub async fn decode_block(&self, block: Block<H256>) {
        let num = block.number.unwrap();
        println!("Decoded block #{}", num);
    }

    /// Decode a full transaction, matching the selector against your ABIs
    pub async fn decode_tx(&self, tx: Transaction) {
        // 1) Identify the target contract
        if let Some(to_addr) = tx.to {
            // 2) Extract first 4 bytes of `input` as the selector
            let input_bytes = tx.input.0;
            if input_bytes.len() < 4 {
                println!("— no calldata for tx {:?}", tx.hash);
                return;
            }
            let selector: [u8; 4] = input_bytes[0..4].try_into().unwrap();

            // 3) Try matching against V2 Pool ABI
            if let Some(func) = self
                .abis
                .v2_pool
                .functions()
                // `short_signature` gives the 4-byte selector for a function :contentReference[oaicite:0]{index=0}
                .find(|f| f.short_signature() == selector)
            {
                
                // Decode the remaining bytes into tokens
                match func.decode_input(&input_bytes[4..]) {
                    Ok(tokens) => {
                        println!("V2 Pool  call on {:?}: {:?}", &func.name, tokens);
                    }
                    Err(e) => eprintln!("decode_input error: {:?}", e),
                }
                return;
            }

            // 4) Try matching against V3 Pool ABI
            if let Some(func) = self
                .abis
                .v3_pool
                .functions()
                .find(| f| f.short_signature() == selector)
            {
                let tokens = func.decode_input(&input_bytes[4..]).unwrap();
                println!("V3 Pool call on {:?}: {:?}", &func.name, tokens);
                return;
            }

            // 5) Check ERC-20 `transfer` or `approve` via its ABI
            if let Some(func) = self
                .abis
                .bep_20
                .functions()
                .find(|k| {
                    // match on function name, or selector:
                    (k.name == "transfer" || k.name == "approve") &&
                    k.short_signature() == selector
                })
            {
                let tokens = func.decode_input(&input_bytes[4..]).unwrap();
                println!("ERC-20 `{}` on {:?}: {:?}", func.name, to_addr, tokens);
                return;
            }

            // 6) Otherwise, unknown or malicious
            println!("Unrecognized tx to {:?}, selector 0x{}", to_addr, hex::encode(selector));
        }
    }

}
