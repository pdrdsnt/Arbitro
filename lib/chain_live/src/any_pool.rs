use std::{collections::BTreeMap, str::FromStr};

use alloy_primitives::{
    Address, B256, B512, U160,
    aliases::{I24, U24},
    keccak256,
};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use chain_db::{
    p_any::AnyPoolSled,
    p_config::{V2Config, V3Config, V4Config},
    p_key::{AddId, SledPoolKey},
    p_state::{V2State, V3State},
    p_ticks::PoolWords,
    p_tokens::Tokens,
};
use chain_json::chain_json_model::PoolJsonModel;
use serde::{Deserialize, Serialize};
use sol::sol_types::{
    IUniswapV2Pair::IUniswapV2PairInstance,
    PoolKey,
    StateView::StateViewInstance,
    V3Pool::{V3PoolInstance, slot0Return},
};

use crate::{
    v2_pool::{V2Data, V2Pool},
    v3_pool::{V3Data, V3Pool},
    v4_pool::{V4Data, V4Pool},
};

pub enum AnyPool<P: Provider + Clone> {
    V2(V2Pool<P>),
    V3(V3Pool<P>),
    V4(V4Pool<P>),
}

impl<P: Provider + Clone> AnyPool<P> {
    pub fn to_key(&self) -> AnyPoolKey {
        match self {
            AnyPool::V2(v2_pool) => AnyPoolKey::V2(*v2_pool.contract.address()),
            AnyPool::V3(v3_pool) => AnyPoolKey::V3(*v3_pool.contract.address()),
            AnyPool::V4(v4_pool) => AnyPoolKey::V4(*v4_pool.contract.address(), v4_pool.id),
        }
    }
}

pub enum AnyPoolData {
    V2(V2Data),
    V3(V3Data),
    V4(V4Data),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnyPoolKey {
    V2(Address),
    V3(Address),
    V4(Address, B256),
}
impl AnyPoolKey {
    fn to_sled_key(&self, chain_id: u64) -> SledPoolKey {
        match self {
            crate::any_pool::AnyPoolKey::V2(address) => {
                SledPoolKey::V2(AddId::new(chain_id, *address))
            }
            crate::any_pool::AnyPoolKey::V3(address) => {
                SledPoolKey::V3(AddId::new(chain_id, *address))
            }
            crate::any_pool::AnyPoolKey::V4(address, fixed_bytes) => {
                SledPoolKey::V4(AddId::new(chain_id, *address), *fixed_bytes)
            }
        }
    }
}

impl<P: Provider + Clone> AnyPool<P> {
    pub fn from_json(pool: PoolJsonModel, provider: P) -> Option<AnyPool<P>> {
        match pool {
            PoolJsonModel::V2 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let t0 = Address::from_str(token0.as_str());
                let t1 = Address::from_str(token1.as_str());
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let data = V2Data {
                        name: None,
                        reserves: None,
                        fee: Some(alloy_primitives::aliases::U24::from(fee)),
                        token0: if let Ok(t) = t0 { Some(t) } else { None },
                        token1: if let Ok(t) = t1 { Some(t) } else { None },
                    };
                    let contract = IUniswapV2PairInstance::new(f_addr, provider);
                    Some(AnyPool::V2(V2Pool::new(contract, data)))
                } else {
                    None
                }
            }
            PoolJsonModel::V3 {
                factory,
                address,
                token0,
                token1,
                fee,
            } => {
                let t0 = Address::from_str(token0.as_str());
                let t1 = Address::from_str(token1.as_str());
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let data = V3Data {
                        fee: Some(alloy_primitives::aliases::U24::from(fee)),
                        token0: if let Ok(t) = t0 { Some(t) } else { None },
                        token1: if let Ok(t) = t1 { Some(t) } else { None },
                        slot0: None,
                        liquidity: None,
                        ticks: PoolWords::default(),
                        tick_spacing: None,
                    };
                    let contract = V3PoolInstance::new(f_addr, provider);
                    Some(AnyPool::V3(V3Pool::new(contract, data)))
                } else {
                    None
                }
            }
            PoolJsonModel::V4 {
                factory,
                address,
                token0,
                token1,
                fee,
                spacing,
                hooks,
            } => {
                let Ok(t0) = Address::from_str(token0.as_str()) else {
                    return None;
                };
                let Ok(t1) = Address::from_str(token1.as_str()) else {
                    return None;
                };
                let eth_fee = U24::from(fee);
                let Ok(tick_spacing) = I24::try_from(spacing) else {
                    return None;
                };

                let mut hook = Address::ZERO;
                if let Some(h_str) = hooks {
                    let Ok(h) = Address::from_str(&h_str) else {
                        return None;
                    };
                    hook = h;
                };
                if let Ok(f_addr) = Address::from_str(factory.unwrap().as_str()) {
                    let pool_key = PoolKey {
                        currency0: t0,
                        currency1: t1,
                        fee: eth_fee,
                        tickSpacing: tick_spacing,
                        hooks: hook,
                    };

                    let data = V4Data {
                        slot0: None,
                        liquidity: None,
                        ticks: PoolWords {
                            words: BTreeMap::new(),
                        },
                        pool_key,
                    };

                    let contract = StateViewInstance::new(f_addr, provider);
                    Some(AnyPool::V4(V4Pool::new(contract, data)))
                } else {
                    return None;
                }
            }
        }
    }

    pub fn to_sled_pool(&self, id: u64) -> AnyPoolSled {
        match self {
            AnyPool::V2(v2_pool) => AnyPoolSled::V2(
                id,
                *v2_pool.contract.address(),
                V2Config {
                    name: v2_pool.data.name.clone(),
                    fee: v2_pool.data.fee,
                    token0: v2_pool.data.token0,
                    token1: v2_pool.data.token1,
                },
                V2State {
                    r0: v2_pool.data.reserves.unwrap_or_else(|| (0_u128, 0_u128)).0,
                    r1: v2_pool.data.reserves.unwrap_or_else(|| (0_u128, 0_u128)).1,
                },
            ),
            AnyPool::V3(v3_pool) => {
                let slot0 = &v3_pool.data.slot0;
                AnyPoolSled::V3(
                    id,
                    *v3_pool.contract.address(),
                    V3Config {
                        name: None,
                        fee: v3_pool.data.fee,
                        tick_spacing: v3_pool.data.tick_spacing,

                        token0: v3_pool.data.token0,
                        token1: v3_pool.data.token0,
                    },
                    V3State {
                        tick: if let Some(s) = slot0 {
                            Some(s.tick)
                        } else {
                            None
                        },
                        x96price: if let Some(s) = slot0 {
                            Some(s.sqrtPriceX96)
                        } else {
                            None
                        },
                        liquidity: v3_pool.data.liquidity,
                    },
                    v3_pool.data.ticks.clone(),
                )
            }
            AnyPool::V4(v4_pool) => {
                let slot0 = v4_pool.data.slot0;
                AnyPoolSled::V4(
                    id,
                    *v4_pool.contract.address(),
                    V4Config {
                        fee: v4_pool.data.pool_key.fee,
                        tick_spacing: v4_pool.data.pool_key.tickSpacing,
                        hooks: v4_pool.data.pool_key.hooks,
                        token0: v4_pool.data.pool_key.currency0.into(),
                        token1: v4_pool.data.pool_key.currency1.into(),
                    },
                    V3State {
                        tick: if let Some(s) = slot0 { Some(s.1) } else { None },
                        x96price: if let Some(s) = slot0 { Some(s.0) } else { None },
                        liquidity: v4_pool.data.liquidity,
                    },
                    v4_pool.data.ticks.clone(),
                )
            }
        }
    }

    pub fn from_sled_pool(pool: AnyPoolSled, provider: P) -> Result<Self, ()> {
        match pool {
            AnyPoolSled::V2(_, address, v2_config, v2_pool_state) => Ok(Self::V2(V2Pool {
                contract: IUniswapV2PairInstance::new(address, provider),
                data: V2Data {
                    name: v2_config.name,
                    reserves: Some((v2_pool_state.r0, v2_pool_state.r1)),
                    fee: v2_config.fee,
                    token0: v2_config.token0,
                    token1: v2_config.token1,
                },
            })),
            AnyPoolSled::V3(_, address, v3_config, v3_pool_state, positions) => {
                Ok(AnyPool::V3(V3Pool {
                    contract: V3PoolInstance::new(address, provider),
                    data: V3Data {
                        slot0: Some(slot0Return {
                            sqrtPriceX96: v3_pool_state.x96price.unwrap_or_else(|| U160::ZERO),
                            tick: v3_pool_state.tick.unwrap_or_else(|| I24::ZERO),
                            observationIndex: 0,
                            observationCardinality: 0,
                            observationCardinalityNext: 0,
                            feeProtocol: 0,
                            unlocked: true,
                        }),
                        liquidity: v3_pool_state.liquidity,
                        ticks: positions,
                        tick_spacing: v3_config.tick_spacing,
                        fee: v3_config.fee,
                        token0: v3_config.token0,
                        token1: v3_config.token1,
                    },
                }))
            }
            AnyPoolSled::V4(_, address, v4_config, v3_pool_state, positions) => {
                let ta = v4_config.token0;
                let tb = v4_config.token1;

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
                        ticks: positions,
                        pool_key: pool_key,
                    },
                    id: id,
                }))
            }
        }
    }
}
