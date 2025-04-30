use ethers::{
    core::types::{U256, Address},
    types::{H160, H256},
    abi::{Token, encode},
    providers::{Provider, Http},
};
use ethers_providers::Middleware;
use std::{env::home_dir, fs, path::Path, process::Command};

pub struct AnvilTest {
    pub provider: Provider<Http>,
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: H256,
    pub block_timestamp: u64,
    pub block_gas_limit: u64,
    pub block_base_fee_per_gas: Option<U256>,
}

// Assuming you have some `Trade` struct
pub struct Trade {
    pub token0: H160,
    pub token1: H160,
    pub fee: u32,
    pub version: String,
    pub dex: String,
    pub pool: H160,
    pub from0: bool,
}

pub async fn encode_and_run_forge(
    path: &[Trade],
    amount_in: U256,
    arc_provider: Provider<Http>,
) {
    // Corrected ABI encoding flow
    let mut pool_tokens = Vec::with_capacity(path.len());

    for trade in path {
        let currency0 = Token::Address(trade.token0);
        let currency1 = Token::Address(trade.token1);

        // Check fee range for uint24
        if trade.fee > 16_777_215 {
            panic!("Invalid fee {} exceeds uint24 max", trade.fee);
        }

        let pool_key = Token::Tuple(vec![
            currency0,
            currency1,
            Token::Uint(U256::from(trade.fee)),
            Token::Int(U256::from(0i32)), // tick_spacing
            Token::Address(H160::zero()), // hooks
        ]);

        let pool_type_u8 = match (trade.version.as_str(), trade.dex.to_lowercase().as_str()) {
            ("v2", "pancake") => 1,
            ("v2", _) => 0,
            ("v3", "pancake") => 3,
            ("v3", _) => 2,
            ("v4", "pancake") => 5,
            ("v4", _) => 4,
            _ => panic!("Invalid version/dex combo"),
        };

        pool_tokens.push(Token::Tuple(vec![
            Token::Address(trade.pool),
            pool_key,
            Token::Uint(U256::from(pool_type_u8)),
            Token::Bool(trade.from0),
        ]));
    }

    println!("Encoded PoolSpec count: {}", pool_tokens.len());
    for (i, spec) in pool_tokens.iter().enumerate() {
        println!("Pool {}: {:?}", i, spec);
    }

    // Encode as [PoolSpec[] array, amount_in]
    let payload = encode(&[Token::Array(pool_tokens), Token::Uint(amount_in)]);

    let home = home_dir().expect("No HOME directory defined");
    let args_path = Path::new("forge/script/pools_input.txt");
    fs::write(home.join(args_path), &payload).expect("Failed to write to file");

    println!("Payload written. Length: {}", payload.len());

    let block_number = arc_provider.get_block_number()
        .await
        .expect("Failed to get block number")
        .to_string();

    let forge_dir = home.join("forge");
    let output = Command::new("forge")
        .current_dir(&forge_dir)
        .arg("script")
        .arg("script/ArbitroTester.sol:ArbitroTester")
        .arg("--fork-url")
        .arg("http://localhost:8545")
        .arg("--fork-block-number")
        .arg(block_number)
        .output()
        .expect("Failed to run forge");

    println!("forge stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("forge stderr:\n{}", String::from_utf8_lossy(&output.stderr));
}
