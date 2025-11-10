#[derive(Serialize, Deserialize, Decode, Encode, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolTokens {
    #[bincode(with_serde)]
    pub a: Option<Address>,
    #[bincode(with_serde)]
    pub b: Option<Address>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub enum AnyPoolState {
    V2(V2PoolState),
    V3(V3PoolState),
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct V3PoolState {
    #[bincode(with_serde)]
    pub tick: Option<I24>,
    #[bincode(with_serde)]
    pub x96price: Option<U160>,
    #[bincode(with_serde)]
    pub liquidity: Option<u128>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct V2PoolState {
    #[bincode(with_serde)]
    pub r0: u128,
    #[bincode(with_serde)]
    pub r1: u128,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct PoolWord {
    pub ticks: Vec<PoolTick>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct PoolWords {
    pub words: BTreeMap<i16, PoolWord>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct PoolTick {
    #[bincode(with_serde)]
    pub tick: I24,

    #[bincode(with_serde)]
    pub liquidity_net: Option<i128>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub enum AnyPoolSled {
    V2(
        u64,
        #[bincode(with_serde)] Address,
        V2Config,
        V2PoolState,
        PoolTokens,
    ),
    V3(
        u64,
        #[bincode(with_serde)] Address,
        V3Config,
        V3PoolState,
        PoolTokens,
    ),
    V4(
        u64,
        #[bincode(with_serde)] Address,
        V4Config,
        V3PoolState,
        PoolTokens,
    ),
}

impl AnyPoolSled {
    pub fn from_pool<P: Provider + Clone>(id: u64, pool: AnyPool<P>) -> Self {
        match pool {
            AnyPool::V2(v2_pool) => AnyPoolSled::V2(
                id,
                *v2_pool.contract.address(),
                V2Config {
                    name: v2_pool.data.name,
                    fee: v2_pool.data.fee,
                },
                V2PoolState {
                    r0: v2_pool.data.reserves.unwrap().0,
                    r1: v2_pool.data.reserves.unwrap().0,
                },
                PoolTokens {
                    a: v2_pool.data.token0,
                    b: v2_pool.data.token1,
                },
            ),
            AnyPool::V3(v3_pool) => {
                let slot0 = v3_pool.data.slot0;
                AnyPoolSled::V3(
                    id,
                    *v3_pool.contract.address(),
                    V3Config {
                        name: None,
                        fee: v3_pool.data.fee,
                        tick_spacing: v3_pool.data.tick_spacing,
                    },
                    V3PoolState {
                        tick: if let Some(s) = slot0 { Some(s.1) } else { None },
                        x96price: if let Some(s) = slot0 { Some(s.0) } else { None },
                        liquidity: v3_pool.data.liquidity,
                    },
                    PoolTokens {
                        a: v3_pool.data.token0,
                        b: v3_pool.data.token0,
                    },
                )
            }
            AnyPool::V4(v4_pool) => todo!(),
        }
    }

    pub fn into_pool<P: Provider + Clone>(self, provider: P) -> Result<AnyPool<P>, ()> {
        match self {
            AnyPoolSled::V2(_, address, v2_config, v2_pool_state, tokens) => {
                Ok(AnyPool::V2(V2Pool {
                    contract: IUniswapV2PairInstance::new(address, provider),
                    data: V2Data {
                        name: v2_config.name,
                        reserves: Some((v2_pool_state.r0, v2_pool_state.r1)),
                        fee: v2_config.fee,
                        token0: tokens.a,
                        token1: tokens.b,
                    },
                }))
            }
            AnyPoolSled::V3(_, address, v3_config, v3_pool_state, tokens) => {
                Ok(AnyPool::V3(V3Pool {
                    contract: V3PoolInstance::new(address, provider),
                    data: V3Data {
                        slot0: Some((
                            v3_pool_state.x96price.unwrap_or_default(),
                            v3_pool_state.tick.unwrap_or_default(),
                            0,
                            0,
                            0,
                            0,
                            false,
                        )),
                        liquidity: v3_pool_state.liquidity,
                        ticks: BTreeMap::new(),
                        tick_spacing: v3_config.tick_spacing,
                        fee: v3_config.fee,
                        token0: tokens.a,
                        token1: tokens.b,
                    },
                }))
            }
            AnyPoolSled::V4(_, address, v4_config, v3_pool_state, tokens) => {
                let Some(ta) = tokens.a else {
                    return Err(());
                };

                let Some(tb) = tokens.a else {
                    return Err(());
                };

                let pool_key = PoolKey {
                    currency0: ta,
                    currency1: tb,
                    fee: v4_config.fee,
                    tickSpacing: v4_config.tick_spacing,
                    hooks: v4_config.hooks,
                };
                let id = keccak256(pool_key.abi_encode());

                Ok(AnyPool::V4(V4Pool {
                    contract: StateViewInstance::new(address, provider),

                    data: V4Data {
                        slot0: Some((
                            v3_pool_state.x96price.unwrap_or_default(),
                            v3_pool_state.tick.unwrap_or_default(),
                            U24::ZERO,
                            U24::ZERO,
                        )),
                        liquidity: v3_pool_state.liquidity,
                        ticks: BTreeMap::new(),
                        pool_key: pool_key,
                    },
                    id: id,
                }))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct V2Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct V3Config {
    pub name: Option<String>,
    #[bincode(with_serde)]
    pub fee: Option<U24>,
    #[bincode(with_serde)]
    pub tick_spacing: Option<I24>,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug)]
pub struct V4Config {
    #[bincode(with_serde)]
    pub fee: U24,
    #[bincode(with_serde)]
    pub tick_spacing: I24,
    #[bincode(with_serde)]
    pub hooks: Address,
}
