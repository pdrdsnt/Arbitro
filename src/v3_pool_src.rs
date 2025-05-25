use std::{collections::HashMap, str::FromStr, sync::Arc};

use ethers::{abi::Address, contract::{Contract, ContractError}, types::{H160, U256}};
use ethers_providers::Provider;
use futures::future::join_all;
use tokio::sync::RwLock;

use crate::{err::PoolUpdateError, mult_provider::MultiProvider, tick_math::Tick, token::Token, trade::Trade, v3_pool_sim::V3PoolSim};

#[derive(Debug)]
pub struct V3PoolSrc {
    pub address: Address,
    pub token0: Arc<RwLock<Token>>,
    pub token1: Arc<RwLock<Token>>,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub current_tick: i32,
    pub active_ticks: Vec<Tick>,
    pub tick_spacing: i32,
    pub liquidity: U256,
    pub x96price: U256,
    pub contract: Contract<Provider<MultiProvider>>,
}

impl V3PoolSrc {
    pub async fn new(
        address: Address,
        token0: Arc<RwLock<Token>>,
        token1: Arc<RwLock<Token>>,
        dex: String,
        version: String,
        fee: u32,
        contract: Contract<Provider<MultiProvider>>,
    ) -> Self {
        let mut instance = V3PoolSrc {
            address,
            token0,
            token1,
            exchange: dex,
            version,
            fee,
            current_tick: 0,
            active_ticks: Vec::new(),
            tick_spacing: 0,
            liquidity: U256::zero(),
            x96price: U256::zero(),
            contract,
        };

        instance.fee = fee;

        let spacing_call = instance.contract.method::<(), i32>("tickSpacing", ());
        let spacing_call_result = spacing_call.unwrap().call_raw().await;
        let spacing = match spacing_call_result {
            Ok(spacing) => spacing,
            Err(erro) => {
                println!("abi erro {}", erro);
                return instance
            }
        };
        println!("tick spacing {:?}", spacing);
        instance.tick_spacing = spacing;

        instance.update().await;
        instance
    }

    pub async fn update(&mut self) -> Result<H160, PoolUpdateError> {
        let slot0_call_result = self
            .contract
            .method::<(), (U256, i32, U256, U256, U256, U256, bool)>("slot0", ());
        match slot0_call_result {
            Ok(slot0) => {
                let result = slot0.call_raw().await;

                match result {
                    Ok((
                        x96price,
                        tick,
                        _observationIdx,
                        _observationCardiality,
                        _observationCardialityNext,
                        _feeProtocol,
                        _unlocked,
                    )) => {
                        self.x96price = x96price;
                        self.current_tick = tick;
                    }
                    Err(_erro) => {
                        // // println!("contract call error {}", erro);
                    }
                }
            }
            Err(_erro) => {
                // // println!("abi erro {}", erro);
            }
        }

        let liquidity_call_result = self.contract.method::<(), (U256)>("liquidity", ());
        match liquidity_call_result {
            Ok(liquidity) => {
                let var = liquidity.call_raw().await;
                println!("liquidity {:?}", var);
                match var {
                    Ok(liquidity) => {
                        self.liquidity = liquidity;
                    }
                    Err(_erro) => {
                        // // println!("contract call error {}", erro);
                    }
                }
            }
            Err(_erro) => {
                // // println!("abi erro {}", erro);
            }
        }

        V3PoolSrc::update_active_ticks(self).await;

        Ok(self.address)
    }

    async fn fetch_bitmap_words(
        contract: &Contract<Provider<MultiProvider>>,
        word_indices: &[i32],
    ) -> HashMap<i32, U256> {
        let futures = word_indices.iter().map(|&idx| async move {
            match Self::fetch_bitmap_word(contract, idx as i16).await {
                Ok(word) => Some((idx, word)), // Store the result if successful
                Err(_) => {
                    // println!("Failed to fetch word for index {}", idx);
                    None
                }
            }
        });

        let results = join_all(futures).await;
        results.into_iter().flatten().collect() // Convert `Option` to `HashMap`
    }

    /// Fetch a single bitmap word from the contract.
    async fn fetch_bitmap_word(
        contract: &Contract<Provider<MultiProvider>>,
        index: i16,
    ) -> Result<U256, ContractError<Provider<MultiProvider>>> {
        contract
            .method::<(i16), U256>("tickBitmap", (index))?
            .call()
            .await
    }

    /// Find the nearest initialized ticks around the current tick.
    async fn find_nearest_ticks(
        contract: &Contract<Provider<MultiProvider>>,
        current_tick: i32,
        tick_spacing: i32,
    ) -> Vec<i32> {
        let normalized_tick = current_tick.div_euclid(tick_spacing);
        let current_word_idx = normalized_tick.div_euclid(256);
        let range = 1;
        // Generate word indices to check (-16 to +16 words from current)
        let word_indices: Vec<i32> = (-range..=range)
            .map(|offset| current_word_idx + offset)
            .collect();

        let word_map = Self::fetch_bitmap_words(contract, &word_indices).await;

        let mut ticks = Vec::new();

        // DIRECTLY ITERATE THROUGH ALL WORDS AND BITS
        for (&word_idx, bitmap) in word_map.iter() {
            if bitmap.is_zero() {
                continue;
            }

            // Check all 256 bits in parallel
            for bit_idx in 0..256 {
                if bitmap.bit(bit_idx) {
                    let normalized = (word_idx * 256) + bit_idx as i32;
                    let tick = normalized * tick_spacing;
                    ticks.push(tick);
                }
            }
        }

        ticks
    }

    async fn fetch_tick_data(contract: &Contract<Provider<MultiProvider>>, ticks: &[i32]) -> Vec<Tick> {
        let tick_futures = ticks
            .iter()
            .map(|&tick| {
                // Create an async block for each tick
                async move {
                    // Setup the method call for the tick.
                    match contract
                        .method::<_, (u128, i128, U256, U256, i64, U256, u32, bool)>("ticks", tick)
                    {
                        Ok(call) => {
                            // Await the call
                            match call.call_raw().await {
                                Ok((_, liquidity_net, _, _, _, _, _, _)) => {
                                    // println!("fetching tick {}", tick);
                                    Some(Tick {
                                        tick,
                                        liquidityNet: liquidity_net,
                                    })
                                }
                                Err(err) => {
                                    // println!("Error fetching tick data for {}: {}", tick, err);
                                    None
                                }
                            }
                        }
                        Err(err) => {
                            // println!("Method call setup failed for tick {}: {}", tick, err);
                            None
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        // Execute all futures concurrently.
        let tick_results = join_all(tick_futures).await;

        // Filter out any results that failed (returned None)
        tick_results
            .into_iter()
            .filter_map(|result| result)
            .collect()
    }

    async fn update_active_ticks(&mut self) {

        let nearest_ticks =
            V3PoolSrc::find_nearest_ticks(&self.contract, self.current_tick, self.tick_spacing).await;

        self.active_ticks = V3PoolSrc::fetch_tick_data(&self.contract, &nearest_ticks).await;

        self.active_ticks.sort_by_key(|t| t.tick);
    }

    pub async fn into_sim(
        &self)
      -> V3PoolSim {
        V3PoolSim::new(
            self.address.clone(),
            self.fee.clone(),
            self.exchange.clone(),
            self.version.clone(),
            self.token0.read().await.clone(),
            self.token1.read().await.clone(),
            self.tick_spacing,
            self.active_ticks.clone(),
            self.liquidity,
            self.x96price,
        )
    }
}


