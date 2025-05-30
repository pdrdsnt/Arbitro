use std::{env::home_dir, fs, path::Path, process::Command};

use ethers::{
    abi::{encode, InvalidOutputType, Token, Tokenizable},
    core::types::{Address, U256},
    providers::{Http, Provider},
    types::{H160, H256},
};
use ethers_providers::Middleware;

use crate::trade::Trade;

pub struct AnvilTest {
    pub provider: Provider<Http,>,
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: H256,
    pub block_timestamp: u64,
    pub block_gas_limit: u64,
    pub block_base_fee_per_gas: Option<U256,>,
}

pub async fn encode_and_run_forge(path: &[Trade], amount_in: U256, arc_provider: Provider<Http,>,) {
    // Corrected ABI encoding flow
    let mut pool_tokens = Vec::with_capacity(path.len(),);

    for trade in path {
        let currency0 = Token::Address(trade.token0,);
        let currency1 = Token::Address(trade.token1,);

        // Check fee range for uint24
        if trade.fee > 16_777_215 {
            panic!("Invalid fee {} exceeds uint24 max", trade.fee);
        }

        let pool_key = Token::Tuple(vec![
            currency0,
            currency1,
            Token::Uint(U256::from(trade.fee,),),
            Token::Int(U256::from(0i32,),), // tick_spacing
            Token::Address(H160::zero(),),  // hooks
        ],);

        let pool_type_u8 = match (trade.version.as_str(), trade.dex.to_lowercase().as_str(),) {
            ("v2", "pancake",) => 1,
            ("v2", _,) => 0,
            ("v3", "pancake",) => 3,
            ("v3", _,) => 2,
            ("v4", "pancake",) => 5,
            ("v4", _,) => 4,
            _ => panic!("Invalid version/dex combo"),
        };

        pool_tokens.push(Token::Tuple(vec![
            Token::Address(trade.pool,),
            pool_key,
            Token::Uint(U256::from(pool_type_u8,),),
            Token::Bool(trade.from0,),
        ],),);
    }

    println!("Encoded PoolSpec count: {}", pool_tokens.len());
    for (i, spec,) in pool_tokens.iter().enumerate() {
        println!("Pool {}: {:?}", i, spec);
    }

    // Encode as [PoolSpec[] array, amount_in]
    let payload = encode(&[Token::Array(pool_tokens,), Token::Uint(amount_in,),],);

    let home = home_dir().expect("No HOME directory defined",);
    let args_path = Path::new("forge/script/pools_input.txt",);
    fs::write(home.join(args_path,), &payload,).expect("Failed to write to file",);

    println!("Payload written. Length: {}", payload.len());

    let block_number = arc_provider.get_block_number().await.expect("Failed to get block number",).to_string();

    let forge_dir = home.join("forge",);
    let output = Command::new("forge",)
        .current_dir(&forge_dir,)
        .arg("script",)
        .arg("script/ArbitroTester.sol:ArbitroTester",)
        .arg("--fork-url",)
        .arg("http://localhost:8545",)
        .arg("--fork-block-number",)
        .arg(block_number,)
        .output()
        .expect("Failed to run forge",);

    println!("forge stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("forge stderr:\n{}", String::from_utf8_lossy(&output.stderr));
}

#[derive(serde::Serialize,)]
enum PoolType {
    UNISWAP_V2,
    PANCAKE_V2,
    UNISWAP_V3,
    PANCAKE_V3,
    UNISWAP_V4,
    PANCAKE_V4,
}

impl Tokenizable for PoolType {
    fn into_token(self,) -> Token {
        match self {
            PoolType::UNISWAP_V2 => Token::Uint(U256::from(0,),),
            PoolType::PANCAKE_V2 => Token::Uint(U256::from(1,),),
            PoolType::UNISWAP_V3 => Token::Uint(U256::from(2,),),
            PoolType::PANCAKE_V3 => Token::Uint(U256::from(3,),),
            PoolType::UNISWAP_V4 => Token::Uint(U256::from(4,),),
            PoolType::PANCAKE_V4 => Token::Uint(U256::from(5,),),
        }
    }

    fn from_token(token: Token,) -> Result<Self, InvalidOutputType,> {
        let value = token.into_uint().ok_or(InvalidOutputType("Expected uint for PoolType".to_string(),),)?;
        match value.as_u32() {
            0 => Ok(PoolType::UNISWAP_V2,),
            1 => Ok(PoolType::PANCAKE_V2,),
            2 => Ok(PoolType::UNISWAP_V3,),
            3 => Ok(PoolType::PANCAKE_V3,),
            4 => Ok(PoolType::UNISWAP_V4,),
            5 => Ok(PoolType::PANCAKE_V4,),
            _ => Err(InvalidOutputType("Invalid PoolType".to_string(),),),
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
    fn into_token(self,) -> Token {
        Token::Tuple(vec![
            Token::Address(self.addr,),
            self.key.into_token(),
            self.pool_typ.into_token(),
            Token::Bool(self.from0,),
        ],)
    }

    fn from_token(token: Token,) -> Result<Self, InvalidOutputType,> {
        let tuple = token.into_tuple().ok_or(InvalidOutputType("Expected tuple for PoolSpec".to_string(),),)?;
        let mut elements = tuple.into_iter();

        Ok(PoolSpec {
            addr: elements
                .next()
                .ok_or(InvalidOutputType("Missing addr".to_string(),),)?
                .into_address()
                .ok_or(InvalidOutputType("Invalid addr type".to_string(),),)?,
            key: PoolKey::from_token(elements.next().ok_or(InvalidOutputType("Missing key".to_string(),),)?,)?,
            pool_typ: PoolType::from_token(
                elements.next().ok_or(InvalidOutputType("Missing pool_typ".to_string(),),)?,
            )?,
            from0: elements
                .next()
                .ok_or(InvalidOutputType("Missing from0".to_string(),),)?
                .into_bool()
                .ok_or(InvalidOutputType("Invalid from0 type".to_string(),),)?,
        },)
    }
}
#[derive(Clone, Debug,)]
struct Currency {
    // Assuming Currency has its own fields, adjust accordingly
    is_token: bool,
    value: Address,
}

impl Tokenizable for Currency {
    fn into_token(self,) -> Token {
        // Adjust based on actual Currency structure
        Token::Tuple(vec![Token::Bool(self.is_token,), Token::Address(self.value,)],)
    }

    fn from_token(token: Token,) -> Result<Self, InvalidOutputType,> {
        let tuple = token.into_tuple().ok_or(InvalidOutputType("Expected tuple for Currency".to_string(),),)?;
        let mut elements = tuple.into_iter();

        Ok(Currency {
            is_token: elements
                .next()
                .ok_or(InvalidOutputType("Missing is_token".to_string(),),)?
                .into_bool()
                .ok_or(InvalidOutputType("Invalid is_token type".to_string(),),)?,
            value: elements
                .next()
                .ok_or(InvalidOutputType("Missing value".to_string(),),)?
                .into_address()
                .ok_or(InvalidOutputType("Invalid value type".to_string(),),)?,
        },)
    }
}

#[derive(Clone, Debug,)]
struct PoolKey {
    currency0: Currency,
    currency1: Currency,
    fee: u32,         // uint24
    tickSpacing: i32, // int24
    hooks: Address,   // IHooks is represented as Address
}
impl Tokenizable for PoolKey {
    fn into_token(self,) -> Token {
        Token::Tuple(vec![
            self.currency0.into_token(),
            self.currency1.into_token(),
            Token::Uint(U256::from(self.fee,),),
            Token::Int(int_to_uint256(self.tickSpacing,),),
            Token::Address(self.hooks,),
        ],)
    }

    fn from_token(token: Token,) -> Result<Self, InvalidOutputType,> {
        let tuple = token.into_tuple().ok_or(InvalidOutputType("Expected tuple for PoolKey".to_string(),),)?;
        let mut elems = tuple.into_iter();

        let c0 = Currency::from_token(elems.next().ok_or(InvalidOutputType("Missing currency0".into(),),)?,)?;
        let c1 = Currency::from_token(elems.next().ok_or(InvalidOutputType("Missing currency1".into(),),)?,)?;

        // fee
        let raw_fee = elems
            .next()
            .and_then(|t| t.into_uint(),)
            .ok_or(InvalidOutputType("Missing or invalid `fee`".into(),),)?
            .as_u32();
        let fee = require_range_u32(raw_fee, 0xFF_FFFF, "fee",)?;

        // tickSpacing
        let raw_tick = elems
            .next()
            .and_then(|t| t.into_int(),)
            .ok_or(InvalidOutputType("Missing or invalid `tickSpacing`".into(),),)?
            .as_u64() as i32;
        let tickSpacing = require_range_i32(raw_tick, -0x800000, 0x7F_FFFF, "tickSpacing",)?;

        let hooks = elems
            .next()
            .and_then(|t| t.into_address(),)
            .ok_or(InvalidOutputType("Missing or invalid `hooks`".into(),),)?;

        Ok(PoolKey {
            currency0: c0,
            currency1: c1,
            fee,
            tickSpacing,
            hooks,
        },)
    }
}
#[derive(Clone, Debug,)]
struct Hooks(Address,);

impl From<Address,> for Hooks {
    fn from(addr: Address,) -> Self { Hooks(addr,) }
}

// Implement any needed trait conversions
impl Tokenizable for Hooks {
    fn into_token(self,) -> Token { Token::Address(self.0,) }

    fn from_token(token: Token,) -> Result<Self, InvalidOutputType,> {
        Ok(Hooks(token.into_address().ok_or(InvalidOutputType("".to_string(),),)?,),)
    }
}

fn require_range_u32(x: u32, max: u32, name: &str,) -> Result<u32, InvalidOutputType,> {
    if x <= max {
        Ok(x,)
    } else {
        Err(InvalidOutputType(format!("{} out of uint24 range", name),),)
    }
}

fn require_range_i32(x: i32, lo: i32, hi: i32, name: &str,) -> Result<i32, InvalidOutputType,> {
    if (lo..=hi).contains(&x,) {
        Ok(x,)
    } else {
        Err(InvalidOutputType(format!("{} out of int24 range", name),),)
    }
}

fn int_to_uint256(x: i32,) -> U256 {
    if x >= 0 {
        U256::from(x as u64,)
    } else {
        // two’s‑complement in 256‑bit space:
        let mag = U256::from((-x) as u64,);
        (!mag).overflowing_add(U256::one(),).0
    }
}
