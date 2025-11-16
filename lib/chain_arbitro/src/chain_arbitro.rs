use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Add,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
};

use alloy::{primitives::Address, providers::Provider};

use chains::{chain::Chain, chain_json_model::TokenJsonModel, token::Token};
use chains_db::{
    chains_db::ChainsDB,
    sled_pool_key::{SledPairKey, SledPoolCollection},
};
use dexes::{
    any_factory::AnyFactory,
    any_pool::{AnyPool, AnyPoolKey},
    v2_factory::V2Factory,
    v2_pool::{V2Data, V2Pool},
    v3_factory::V3Factory,
    v3_pool::{V3Data, V3Pool},
    v4_factory::V4Factory,
    v4_pool::{V4Data, V4Pool},
};
use futures::future::join_all;
use sol::sol_types::{
    IUniswapV2Factory::IUniswapV2FactoryInstance, IUniswapV2Pair::IUniswapV2PairInstance,
    IUniswapV3Factory, StateView::StateViewInstance, V3Pool::V3PoolInstance,
};
use tokio::io::join;

pub struct ChainArbitro<P: Provider + Clone> {
    chains_id: u64,
    dexes: Vec<AnyFactory<P>>,
    tokens: HashMap<Address, Token<P>>,
    pools: HashMap<AnyPoolKey, AnyPool<P>>,
    provider: P,
    pools_by_tokens: HashMap<Address, Vec<AnyPoolKey>>,
    db: &ChainsDB,
}

impl<P: Provider + Clone> ChainArbitro<P> {
    pub fn from_chain(chain: &Chain, provider: P, db: ChainsDB) -> Self {
        let mut dexes: Vec<AnyFactory<P>> = Vec::new();

        let mut tokens: HashMap<Address, Token<P>> = HashMap::new();

        for c in &chain.tokens {
            let token = Token::from_token_model(&c, provider.clone());
            if let Ok(addr) = Address::from_str(&c.address) {
                tokens.insert(addr, token);
            }
        }

        for c in &chain.dexes {
            match c {
                chains::chain_json_model::DexJsonModel::V2 {
                    address,
                    fee,
                    stable_fee,
                } => {
                    if let Ok(addr) = Address::from_str(address.as_str()) {
                        let contract = IUniswapV2FactoryInstance::new(addr, provider.clone());
                        let factory = AnyFactory::V2(V2Factory {
                            name: "lost".to_string(),
                            contract: contract,
                            fee: alloy::primitives::aliases::U24::from(*fee),
                        });
                        dexes.push(factory);
                    }
                }

                chains::chain_json_model::DexJsonModel::V3 { address, fee } => {
                    if let Ok(addr) = Address::from_str(address.as_str()) {
                        let contract = IUniswapV3Factory::new(addr, provider.clone());

                        let factory = AnyFactory::V3(V3Factory {
                            name: "lost".to_string(),
                            contract: contract,
                        });
                        dexes.push(factory);
                    }
                }

                chains::chain_json_model::DexJsonModel::V4 { address } => {
                    if let Ok(addr) = Address::from_str(address.as_str()) {
                        let contract = StateViewInstance::new(addr, provider.clone());
                        let factory = AnyFactory::V4(V4Factory {
                            name: "lost".to_string(),
                            contract: contract,
                        });
                        dexes.push(factory);
                    }
                }
            }
        }

        let mut pools_by_token: HashMap<Address, Vec<AnyPoolKey>> = HashMap::new();
        let mut pools = HashMap::<AnyPoolKey, AnyPool<P>>::new();

        for (k, p) in &chain.pools {
            let a = p.iter().for_each(|x| {
                if let Some(pool) = x.clone().to_pool(provider.clone()) {
                    if let Some(pool_key) = x.clone().to_key() {
                        let mut pool_tokens: (Option<Address>, Option<Address>);

                        match &pool {
                            AnyPool::V2(v2_pool) => {
                                pool_tokens = (v2_pool.data.token0, v2_pool.data.token1)
                            }
                            AnyPool::V3(v3_pool) => {
                                pool_tokens = (v3_pool.data.token0, v3_pool.data.token1)
                            }
                            AnyPool::V4(v4_pool) => {
                                pool_tokens = (
                                    v4_pool.data.pool_key.currency0.into(),
                                    v4_pool.data.pool_key.currency1.into(),
                                )
                            }
                        }

                        pools.insert(pool_key.clone(), pool);

                        if let Some(t0) = pool_tokens.0 {
                            pools_by_token
                                .entry(t0)
                                .and_modify(|x| x.push(pool_key.clone()))
                                .or_insert(vec![pool_key.clone()]);
                        }
                        if let Some(t1) = pool_tokens.1 {
                            pools_by_token
                                .entry(t1)
                                .and_modify(|x| x.push(pool_key.clone()))
                                .or_insert(vec![pool_key.clone()]);
                        }
                    }
                }
            });
        }

        ChainArbitro {
            chains_id: chain.id,
            dexes: dexes,
            tokens: tokens,
            provider: provider,
            pools: pools,
            pools_by_tokens: pools_by_token,
            db,
        }
    }

    pub async fn insert_token(&mut self, token: Token<P>) {
        let addr = *token.contract.address();
        self.tokens.insert(addr, token);

        let (v2, v3, v4) = self.get_token_pools_calls(addr);

        let results = tokio::join!(join_all(v2), join_all(v3), join_all(v4));

        let mut new_pools = Vec::new();

        for data in results.0 {
            if let Some(v2_data) = data {
                let contract = IUniswapV2PairInstance::new(v2_data.0, self.provider.clone());

                let t0: Option<Address> = v2_data.1.token0;
                let t1: Option<Address> = v2_data.1.token1;

                let new_pool = V2Pool::new(contract, v2_data.1);
                let any_pool = AnyPool::V2(new_pool);
                new_pools.push((t0, t1, any_pool));
            }
        }

        for _data in results.1 {
            for data in _data {
                let t0: Option<Address> = data.1.token0;
                let t1: Option<Address> = data.1.token1;

                let contract = V3PoolInstance::new(data.0, self.provider.clone());
                let new_pool = V3Pool::new(contract, data.1);
                let any_pool = AnyPool::V3(new_pool);
                new_pools.push((t0, t1, any_pool));
            }
        }

        for _data in results.2 {
            for data in _data {
                let new_pool = data;
                let t0: Address = new_pool.data.pool_key.currency0.into();
                let t1: Address = new_pool.data.pool_key.currency1.into();
                let any_pool = AnyPool::V4(new_pool);
                new_pools.push((Some(t0), Some(t1), any_pool));
            }
        }

        for (_t0, _t1, pool) in new_pools {
            let key = pool.to_key();
            if let Some(t0) = _t0 {
                let key = self
                    .pools_by_tokens
                    .entry(t0)
                    .and_modify(|x| x.push(key.clone()))
                    .or_insert(vec![key.clone()]);
            }

            if let Some(t1) = _t1 {
                let key = self
                    .pools_by_tokens
                    .entry(t1)
                    .and_modify(|x| x.push(key.clone()))
                    .or_insert(vec![key.clone()]);
            }
        }
    }

    pub fn get_token_pools_calls(
        &self,
        addr: Address,
    ) -> (
        Vec<impl Future<Output = Option<(Address, V2Data)>>>,
        Vec<impl Future<Output = Vec<(Address, V3Data)>>>,
        Vec<impl Future<Output = Vec<V4Pool<P>>>>,
    ) {
        let mut pairs = Vec::new();

        let mut v2_calls = Vec::new();
        let mut v3_calls = Vec::new();
        let mut v4_calls = Vec::new();

        self.tokens
            .iter()
            .filter_map(|x| if x.0 != &addr { Some(x) } else { None })
            .for_each(|x| pairs.push((*x.0, addr)));

        for pair in pairs.iter() {
            let key = SledPairKey::new(self.chains_id, pair.0, pair.1);

            let my_pools = self
                .db
                .get_pools_with_pair(key)
                .unwrap_or_else(|| SledPoolCollection {
                    inner: HashMap::new(),
                })
                .inner;

            for dex in self.dexes.iter() {
                match dex {
                    AnyFactory::V2(v2_factory) => {
                        let ppp = v2_factory.search_pool(pair.0, pair.1, my_pools);
                        v2_calls.push(ppp);
                    }
                    AnyFactory::V3(v3_factory) => {
                        let ppp = v3_factory.search_pools(pair.0, pair.1, my_pools);
                        v3_calls.push(ppp);
                    }
                    AnyFactory::V4(v4_factory) => {
                        let ppp = v4_factory.search_pools(pair.0, pair.1, my_pools);
                        v4_calls.push(ppp);
                    }
                }
            }
        }

        (v2_calls, v3_calls, v4_calls)
    }

    pub fn remove_token() {}
}
