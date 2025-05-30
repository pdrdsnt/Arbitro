use ethers::{
    abi::{Address, RawLog},
    contract::EthEvent,
    types::{Log, H160, H256, I256, U256},
    utils::keccak256,
};

#[derive(Debug,)]
pub enum PoolAction {
    // V2 actions
    SwapV2 {
        sender: H160,
        amount0_in: U256,
        amount1_in: U256,
        amount0_out: U256,
        amount1_out: U256,
        to: H160,
    },
    MintV2 {
        sender: H160,
        amount0: U256,
        amount1: U256,
    },
    BurnV2 {
        sender: H160,
        amount0: U256,
        amount1: U256,
        to: H160,
    },
    // V3 actions
    SwapV3 {
        sender: H160,
        recipient: H160,
        amount0: I256,
        amount1: I256,
        sqrt_price_x96: U256,
        liquidity: U256,
        tick: i32,
    },
    MintV3 {
        sender: H160,
        owner: H160,
        tick_lower: i32,
        tick_upper: i32,
        amount: U256,
        amount0: U256,
        amount1: U256,
    },
    BurnV3 {
        owner: H160,
        tick_lower: i32,
        tick_upper: i32,
        amount: U256,
        amount0: U256,
        amount1: U256,
    },
}

impl PoolAction {
    pub fn parse_pool_action(log: &Log,) -> Option<(PoolAction,H160),> {
        // V2 signatures
        let swap_v2_sig = H256::from_slice(&keccak256(b"Swap(address,uint256,uint256,uint256,uint256,address)",),);
        let mint_v2_sig = H256::from_slice(&keccak256(b"Mint(address,uint256,uint256)",),);
        let burn_v2_sig = H256::from_slice(&keccak256(b"Burn(address,uint256,uint256,address)",),);
        // V3 signatures
        let swap_v3_sig = H256::from_slice(&keccak256(b"Swap(address,address,int256,int256,uint160,uint128,int24)",),);
        let mint_v3_sig = H256::from_slice(&keccak256(b"Mint(address,address,int24,int24,uint128,uint256,uint256)",),);
        let burn_v3_sig = H256::from_slice(&keccak256(b"Burn(address,int24,int24,uint128,uint256,uint256)",),);

        // Prepare RawLog for decoding
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.to_vec(),
        };

        let mut top =
        match log.topics.get(0,)? {
            // V2
            topic if topic == &swap_v2_sig => {
                let e = SwapEventV2::decode_log(&raw_log,).ok()?;
                Some(PoolAction::SwapV2 {
                    sender: e.sender,
                    amount0_in: e.amount0_in,
                    amount1_in: e.amount1_in,
                    amount0_out: e.amount0_out,
                    amount1_out: e.amount1_out,
                    to: e.to,
                },)
            },
            topic if topic == &mint_v2_sig => {
                let e = MintEventV2::decode_log(&raw_log,).ok()?;
                Some(PoolAction::MintV2 {
                    sender: e.sender,
                    amount0: e.amount0,
                    amount1: e.amount1,
                },)
            },
            topic if topic == &burn_v2_sig => {
                let e = BurnEventV2::decode_log(&raw_log,).ok()?;
                Some(PoolAction::BurnV2 {
                    sender: e.sender,
                    amount0: e.amount0,
                    amount1: e.amount1,
                    to: e.to,
                },)
            },
            // V3
            topic if topic == &swap_v3_sig => {
                let e = SwapEventV3::decode_log(&raw_log,).ok()?;
                Some(PoolAction::SwapV3 {
                    sender: e.sender,
                    recipient: e.recipient,
                    amount0: e.amount0,
                    amount1: e.amount1,
                    sqrt_price_x96: e.sqrt_price_x96,
                    liquidity: e.liquidity,
                    tick: e.tick,
                },)
            },
            topic if topic == &mint_v3_sig => {
                let e = MintEventV3::decode_log(&raw_log,).ok()?;
                Some(PoolAction::MintV3 {
                    sender: e.sender,
                    owner: e.owner,
                    tick_lower: e.tick_lower,
                    tick_upper: e.tick_upper,
                    amount: e.amount,
                    amount0: e.amount0,
                    amount1: e.amount1,
                },)
            },
            topic if topic == &burn_v3_sig => {
                let e = BurnEventV3::decode_log(&raw_log,).ok()?;
                Some(PoolAction::BurnV3 {
                    owner: e.owner,
                    tick_lower: e.tick_lower,
                    tick_upper: e.tick_upper,
                    amount: e.amount,
                    amount0: e.amount0,
                    amount1: e.amount1,
                },)
            },
            _ => None,
        };

        if let Some(r) = top {
            Some((r,log.address))
        }else {
            None
        }

    }
}

// V2 event structs
#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Swap", abi = "Swap(address,uint256,uint256,uint256,uint256,address)")]
struct SwapEventV2 {
    pub sender: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
    pub to: Address,
}

#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Mint", abi = "Mint(address,uint256,uint256)")]
struct MintEventV2 {
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Burn", abi = "Burn(address,uint256,uint256,address)")]
struct BurnEventV2 {
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub to: Address,
}

// V3 event structs
#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Swap", abi = "Swap(address,address,int256,int256,uint160,uint128,int24)")]
struct SwapEventV3 {
    pub sender: Address,
    pub recipient: Address,
    pub amount0: I256,
    pub amount1: I256,
    pub sqrt_price_x96: U256,
    pub liquidity: U256,
    pub tick: i32,
}

#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Mint", abi = "Mint(address,address,int24,int24,uint128,uint256,uint256)")]
struct MintEventV3 {
    pub sender: Address,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone, EthEvent,)]
#[ethevent(name = "Burn", abi = "Burn(address,int24,int24,uint128,uint256,uint256)")]
struct BurnEventV3 {
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
}
