use crate::{
    pool_utils::{Tick, Trade},
    token::Token,
};
use axum::http::version;
use bigdecimal::{self, BigDecimal, FromPrimitive};
use ethers::{
    abi::Address,
    contract::{Contract, ContractError},
    core::k256::elliptic_curve::{
        bigint,
        consts::{U16, U160, U2, U24, U25, U8},
    },
    providers::{Http, Provider},
    types::{H160, U256},
};
use futures::future::join_all;
use num_traits::{float, Float};
use std::ops::{BitAnd, Mul};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ptr::read,
    str::FromStr,
};
pub trait Pool {
    async fn update(&mut self);
    fn trade(&self, amount_in: U256, from: bool) -> Option<Trade>;
}

#[derive(Debug)]
pub struct V2Pool {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub reserves0: U256,
    pub reserves1: U256,
    pub contract: Contract<Provider<Http>>,
}

impl V2Pool {
    // Private constructor
    async fn new(
        exchange: String,
        version: String,
        fee: u32,
        address: Address,
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Http>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange,
            version,
            fee,
            reserves0: U256::from(0),
            reserves1: U256::from(0),
            contract,
        }
    }

    pub async fn new_with_update(
        exchange: String,
        version: String,
        fee: u32,
        address: Address,
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Http>>,
    ) -> Self {
        let mut instance =
            V2Pool::new(exchange, version, fee, address, token0, token1, contract).await;
        instance.update().await;
        instance
    }
}

impl Pool for V2Pool {
    async fn update(&mut self) {
        let reserves_call_result = self
            .contract
            .method::<(), (U256, U256, U256)>("getReserves", ());
        match reserves_call_result {
            Ok(reserves) => {
                let var = reserves.call_raw().await;

                match var {
                    Ok((reserve0, reserve1, time)) => {
                        self.reserves0 = reserve0;
                        self.reserves1 = reserve1;
                    }
                    Err(erro) => println!("contract call error {}", erro),
                }
            }
            Err(erro) => println!("abi erro {}", erro),
        }
    }

    fn trade(&self, amount_in: U256, from0: bool) -> Option<Trade> {
        if (from0 && self.reserves0 == U256::zero()) || (!from0 && self.reserves1 == U256::zero()) {
            return None;
        }
        let big_amount_in = amount_in.clone();
        let r0 = &self.reserves0;
        let r1 = &self.reserves1;

        // Calculate the input and output reserves based on the trade direction
        let (reserve_in, reserve_out) = if from0 { (r0, r1) } else { (r1, r0) };

        // Fee multiplier: assume self.fee is in basis points (e.g., 30 for 0.3%)
        let fee_multiplier = U256::from(self.fee);
        let tenk = U256::from(10_000);

        let amount_after_fee = amount_in
            .checked_mul(tenk.checked_sub(fee_multiplier)?)?
            .checked_div(tenk)?;

        // Calculate amount out using the constant product formula
        // amount_out = (reserve_out * amount_in_with_fee) / (reserve_in + amount_in_with_fee)
        let numerator = reserve_out * &amount_after_fee;
        let denominator = reserve_in + &amount_after_fee;

        if denominator == U256::from(0) {
            return None;
        }

        let amount_out = numerator / denominator;

        // New reserves after the swap
        let (new_r0, new_r1) = if from0 {
            (r0 + amount_after_fee, r1 - amount_out)
        } else {
            (r0 - amount_out, r1 + amount_after_fee)
        };

        // Calculate current and new prices
        let scale = U256::from(1e18 as u64);
        let current_price = if from0 {
            (self.reserves1.checked_mul(scale)?).checked_div(self.reserves0)?
        } else {
            (self.reserves0.checked_mul(scale)?).checked_div(self.reserves1)?
        };
        let new_price = if from0 {
            (new_r1.checked_mul(scale)?).checked_div(new_r0)?
        } else {
            (new_r0.checked_mul(scale)?).checked_div(new_r1)?
        };

        // Price impact
        let price_impact = if current_price > U256::zero() {
            let delta = if current_price > new_price {
                current_price - new_price
            } else {
                new_price - current_price
            };

            delta.checked_mul(scale)?.checked_div(current_price)? // Scales to 1e18 (e.g., 0.1e18 = 10%)
        } else {
            U256::zero()
        };
        // Create the trade data object
        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            fee: self.fee,
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in: big_amount_in.clone(),
            amount_out,
            price_impact,
            fee_amount: &big_amount_in - &amount_after_fee, // Fee amount
            raw_price: current_price,
        })
    }
}

#[derive(Debug)]
pub struct V3Pool {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub current_tick: i32,
    pub active_ticks: Vec<Tick>,
    pub tick_spacing: i32,
    pub liquidity: U256,
    pub x96price: U256,
    pub contract: Contract<Provider<Http>>,
}

impl V3Pool {
    // Private constructor
    async fn new(
        address: Address,
        fee: u32,
        dex: String,
        version: String,
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Http>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: dex,
            version,
            current_tick: 0,
            active_ticks: Vec::new(),
            fee,
            tick_spacing: 0,
            liquidity: U256::from(0),
            x96price: U256::from(0),
            contract,
        }
    }

    pub async fn new_with_update(
        address: Address,
        token0: Token,
        token1: Token,
        dex: String,
        version: String,
        fee: u32,
        contract: Contract<Provider<Http>>,
    ) -> Self {
        let mut instance = Self::new(address, fee, dex, version, token0, token1, contract).await;

        instance.fee = fee;

        let spacing_call = instance.contract.method::<(), i32>("tickSpacing", ());
        let spacing_call_result = spacing_call.unwrap().call_raw().await;
        let spacing = match spacing_call_result {
            Ok(spacing) => spacing,
            Err(erro) => {
                // println!("abi erro {}", erro);
                10
            }
        };
        instance.tick_spacing = spacing;

        instance.update().await;
        instance
    }
}

impl Pool for V3Pool {
    async fn update(&mut self) {
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

        V3Pool::update_active_ticks(self).await;
    }

    fn trade(&self, amount_in: U256, from0: bool) -> Option<Trade> {
        let mut ticks = self.active_ticks.clone();

        //println!("ticks {}", ticks.len());
        if from0 {
            ticks.retain(|&t| t.tick > self.current_tick);
        } else {
            ticks.retain(|&t| t.tick < self.current_tick);
            ticks.reverse();
        };

        let mut remaining_in = U256::from(amount_in);
        let mut total_out = U256::from(0);

        let x96 = U256::from_str("79228162514264337593543950336").unwrap();
        let sqrt_price_bd = &self.x96price.clone();
        let initial_sqrt_price = &sqrt_price_bd;

        let mut current_liquidity = self.liquidity;
        let mut current_tick: i32 = self.current_tick;
        let mut current_sqrt_price = *initial_sqrt_price.clone();

        let fee_percent = U256::from(self.fee);
        let fee_amount = amount_in
            .checked_mul(fee_percent)
            .and_then(|v| v.checked_div(U256::from(1_000_000)))?;

        let mut remaining_in = amount_in.checked_sub(fee_amount)?;
        //println!("starting trade for {}", self.token0.symbol);
        //println!("amount in {}", amount_in);

        let TICK_BASE_NUMERATOR: U256 = U256::from(10001);
        let TICK_BASE_DENOMINATOR: U256 = U256::from(10000);
        let Q96: U256 = U256::from(1) << 96;
        //println!("current tick {}", current_tick);
        //println!("ticks {}", ticks.len());

        for tick in ticks {
            //println!("computing next price");
            let next_sqroot_price = V3Pool::tick_price(tick.tick)?;
            //println!("next price {}", next_sqroot_price);

            let available_liquidity = current_liquidity.clone();
            //println!("compute amount possible");
            let amount_possible = V3Pool::compute_amount_possible(
                from0,
                &available_liquidity,
                &current_sqrt_price,
                &next_sqroot_price,
            )?;
            //println!("amount possible {}", amount_possible);
            //println!("starting trade for tick {}", tick.tick);
            //println!("amount possible {}", amount_possible);

            if remaining_in < amount_possible {
                //println!("remaining in do not cross tick");
                let new_price = if from0 {
                    //println!("computing price from 0");
                    let r = V3Pool::compute_price_from0(
                        &remaining_in,
                        &available_liquidity,
                        &current_sqrt_price,
                        true,
                    );
                    //println!("price from 0 {:?}", r);
                    r?
                } else {
                    //println!("computing price from 1");
                    let r = V3Pool::compute_price_from1(
                        &remaining_in,
                        &available_liquidity,
                        &current_sqrt_price,
                        true,
                    );
                    //println!("price from 1 {:?}", r);
                    r?
                };

                if from0 {
                    let price_diff = current_sqrt_price.checked_sub(new_price)?;
                    let delta_out = available_liquidity
                        .checked_mul(price_diff)?
                        .checked_div(U256::from(1u128 << 96))?;
                    total_out = total_out.checked_add(delta_out)?;
                } else {
                    let inv_p_current = U256::from(1u128 << 96)
                        .checked_mul(U256::from(1u128 << 96))?
                        .checked_div(current_sqrt_price)?;

                    let inv_p_new = U256::from(1u128 << 96)
                        .checked_mul(U256::from(1u128 << 96))?
                        .checked_div(new_price)?;

                    let inv_diff = inv_p_current.checked_sub(inv_p_new)?;
                    let delta_out = available_liquidity
                        .checked_mul(inv_diff)?
                        .checked_div(U256::from(1u128 << 96))?;

                    total_out = total_out.checked_add(delta_out)?;
                }

                // // println!("reamining in in bounds {}", remaining_in);
                remaining_in = U256::from(0);
                break;
            } else {
                let delta_out = if from0 {
                    available_liquidity
                        .checked_mul(next_sqroot_price.checked_sub(current_sqrt_price)?)?
                        .checked_div(U256::from(1u128 << 96))?
                } else {
                    let numerator = available_liquidity
                        .checked_mul(current_sqrt_price.checked_sub(next_sqroot_price)?)?
                        .checked_mul(U256::from(1u128 << 96))?;
                    let denominator = current_sqrt_price.checked_mul(next_sqroot_price)?;
                    numerator.checked_div(denominator)?
                };

                total_out = total_out.checked_add(delta_out)?;
                current_tick = tick.tick;

                let liquidity_net = tick.liquidityNet;
                if from0 {
                    current_liquidity = if liquidity_net > 0 {
                        current_liquidity.saturating_add(U256::from(liquidity_net as u128))
                    } else {
                        current_liquidity.saturating_sub(U256::from((-liquidity_net) as u128))
                    };
                } else {
                    current_liquidity = if liquidity_net < 0 {
                        current_liquidity.saturating_add(U256::from((-liquidity_net) as u128))
                    } else {
                        current_liquidity.saturating_sub(U256::from(liquidity_net as u128))
                    };
                }
                current_sqrt_price = next_sqroot_price;
                remaining_in = remaining_in
                    .checked_sub(amount_possible)
                    .unwrap_or(U256::zero());
            }
        }

        if remaining_in > U256::zero() {
            // // println!("Remaining amount in: {}", remaining_in);
            // // println!("not enough liquidity");
        }

        // // println!("trade simulated in dex {} {}", self.exchange, self.version);
        // // println!("amount_in {}", amount_in);
        // // println!("amount_out {}", total_out);
        // // println!("from {} to {}", self.token0.symbol, self.token1.symbol);

        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            fee: self.fee,
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in,
            amount_out: total_out,
            price_impact: amount_in,
            fee_amount,
            raw_price: total_out,
        })
    }
}

impl V3Pool {
    fn compute_amount_possible(
        from0: bool,
        available_liquidity: &U256,
        current_sqrt_price: &U256,
        next_sqrt_price: &U256,
    ) -> Option<U256> {
        let Q96: U256 = U256::from(1) << 96;
        // println!("Q96 = {}", Q96);

        if from0 {
            // Token0 -> Token1: Δx = (L * Δ√P * Q96) / (sqrtP_current * sqrtP_next)
            let diff = next_sqrt_price.checked_sub(*current_sqrt_price)?;
            // println!("diff (next_sqrt_price - current_sqrt_price) = {}", diff);

            if diff.is_zero() {
                // println!("diff is zero; returning None");
                return None;
            }

            let denom = current_sqrt_price.checked_mul(*next_sqrt_price)?;
            // println!("denom (current_sqrt_price * next_sqrt_price) = {}", denom);

            // Multiply L * Δ√P first.
            let liquidity_mul_diff = available_liquidity.checked_mul(diff)?;
            // println!("available_liquidity * diff = {}", liquidity_mul_diff);

            // Divide by denom.
            let scaled = liquidity_mul_diff.checked_div(denom >> 96)?;
            // println!("scaled ( (L * diff) / denom >> 96 ) = {}", scaled);

            // Multiply by Q96.
            let result = scaled;
            // println!("result (scaled * Q96) = {}", result);

            Some(result)
        } else {
            // Token1 -> Token0: Δy = (L * Δ√P) / Q96
            let diff = current_sqrt_price.checked_sub(*next_sqrt_price)?;
            // println!("diff (current_sqrt_price - next_sqrt_price) = {}", diff);

            if diff.is_zero() {
                // println!("diff is zero; returning None");
                return None;
            }

            let liquidity_mul_diff = available_liquidity.checked_mul(diff)?;
            // println!("available_liquidity * diff = {}", liquidity_mul_diff);

            let result = liquidity_mul_diff.checked_div(Q96)?;
            // println!("result ( (L * diff) / Q96 ) = {}", result);

            Some(result)
        }
    }

    fn compute_price_from0(
        amount: &U256,
        available_liquidity: &U256,
        current_sqrt_price: &U256,
        add: bool,
    ) -> Option<U256> {
        // Debug prints (optional)
        // println!("Inputs:");
        // println!("  Δx (amount): {}", amount);
        // println!("  L (liquidity): {}", available_liquidity);
        // println!("  √P (current_sqrt_price): {}", current_sqrt_price);

        // Step 1: Compute L << 96 (Q96L)
        let Q96L = available_liquidity << (96);
        // println!("Q96L (L << 96): {}", Q96L);

        // Step 2: Compute (L << 96) / √P (scaled_liquidity)
        let scaled_liquidity = Q96L.checked_div(*current_sqrt_price)?;
        // println!("scaled_liquidity (Q96L / √P): {}", scaled_liquidity);

        // Step 3: Compute denominator = scaled_liquidity ± Δx
        let denominator = if add {
            scaled_liquidity.checked_add(*amount)?
        } else {
            scaled_liquidity.checked_sub(*amount)?
        };
        // println!("denominator (scaled_liquidity ± Δx): {}", denominator);

        // Step 4: Compute new_sqrt_price = Q96L / denominator
        let new_sqrt_price = Q96L.checked_div(denominator)?;
        // println!("new_sqrt_price (Q96L / denominator): {}", new_sqrt_price);

        Some(new_sqrt_price)
    }

    fn compute_price_from1(
        amount: &U256,
        available_liquidity: &U256,
        current_sqrt_price: &U256,
        add: bool,
    ) -> Option<U256> {
        // For token1, calculate the difference as (current - next)
        let n = (amount << 96).checked_div(*available_liquidity)?;
        // amount_possible = available_liquidity * diff

        Some(if add {
            current_sqrt_price.checked_add(n)?
        } else {
            current_sqrt_price.checked_sub(n)?
        })
    }
    fn tick_price(target_tick: i32) -> Option<U256> {
        const MAX_TICK: i32 = 887272;
        let abs_tick = target_tick.unsigned_abs() as u32;
    
        if abs_tick > MAX_TICK as u32 {
            eprintln!(
                "[0] Tick {} exceeds maximum allowed (±{})",
                target_tick, MAX_TICK
            );
            return None;
        }
    
        let mut ratio = if abs_tick & 0x1 != 0 {
            U256::from_dec_str("255706422905421325395407485534392863200").unwrap()
        } else {
            U256::from(1) << 128
        };
    
        // Magic numbers are now ordered from highest mask to lowest
        let magic_numbers = [
            (
                0x80000,
                U256::from_dec_str("366325949420163452428643381347626447728").unwrap(),
            ),
            (
                0x40000,
                U256::from_dec_str("435319348045928502739365042735923241779").unwrap(),
            ),
            (
                0x20000,
                U256::from_dec_str("142576269300693600730609870735819320320").unwrap(),
            ),
            (
                0x10000,
                U256::from_dec_str("366325949420163452428643381347626447728").unwrap(),
            ),
            (
                0x8000,
                U256::from_dec_str("844815322999501822113930908203125000000").unwrap(),
            ),
            (
                0x4000,
                U256::from_dec_str("340265210418746478515625000000000000000").unwrap(),
            ),
            (
                0x2000,
                U256::from_dec_str("215416728668509908758128906250000000000").unwrap(),
            ),
            (
                0x1000,
                U256::from_dec_str("177803588050028359909546862144531250000").unwrap(),
            ),
            (
                0x800,
                U256::from_dec_str("170408874814886611515626254292199532339").unwrap(),
            ),
            (
                0x400,
                U256::from_dec_str("170141183460469231731687303715884105728").unwrap(),
            ),
            (
                0x200,
                U256::from_dec_str("3868562622766813359059763198240802791").unwrap(),
            ),
            (
                0x100,
                U256::from_dec_str("29287344681543793554040907002057611822").unwrap(),
            ),
            (
                0x80,
                U256::from_dec_str("115165952705265534866474743471916972268").unwrap(),
            ),
            (
                0x40,
                U256::from_dec_str("191204177664095573937843702857003287777").unwrap(),
            ),
            (
                0x20,
                U256::from_dec_str("234435455086227615880830483505416481938").unwrap(),
            ),
            (
                0x10,
                U256::from_dec_str("250846047417607353339794883300939388931").unwrap(),
            ),
            (
                0x8,
                U256::from_dec_str("254322734553735582512512255949976165369").unwrap(),
            ),
            (
                0x4,
                U256::from_dec_str("255223438104885656517683320344580614584").unwrap(),
            ),
            (
                0x2,
                U256::from_dec_str("255706422905421325395407485534392863200").unwrap(),
            ),
        ];
    
        // Iterate from highest mask to lowest
        for (mask, magic) in magic_numbers.iter().rev() {
            if abs_tick & mask != 0 {
                ratio = match ratio.checked_mul(*magic) {
                    Some(v) => v >> 128,
                    None => {
                        eprintln!(
                            "[1] Multiplication overflow at tick {} (mask: {:x})",
                            target_tick, mask
                        );
                        return None;
                    }
                };
            }
        }
    
        if target_tick < 0 {
            ratio = match U256::max_value().checked_div(ratio) {
                Some(v) => v,
                None => {
                    eprintln!("[2] Division overflow for negative tick {}", target_tick);
                    return None;
                }
            };
        }
    
        let sqrt_price_x96 = (ratio >> 32)
            + if (ratio % (U256::from(1) << 32)).is_zero() {
                U256::from(0)
            } else {
                U256::from(1)
            };
        Some(sqrt_price_x96)
    }
    fn update_liquidity(current: U256, net: i128) -> Option<U256> {
        let _net = U256::from(net);

        if net < i128::from(0) {
            current.checked_sub(_net)
        } else {
            current.checked_sub(_net)
        }
    }

    async fn fetch_bitmap_words(
        contract: &Contract<Provider<Http>>,
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
        contract: &Contract<Provider<Http>>,
        index: i16,
    ) -> Result<U256, ContractError<Provider<Http>>> {
        contract
            .method::<(i16), U256>("tickBitmap", (index))?
            .call()
            .await
    }

    /// Find the nearest initialized ticks around the current tick.
    async fn find_nearest_ticks(
        contract: &Contract<Provider<Http>>,
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

        // Sort by proximity to current tick
        ticks.sort_by_key(|&t| (t - current_tick).abs());
        ticks
    }

    async fn fetch_tick_data(contract: &Contract<Provider<Http>>, ticks: &[i32]) -> Vec<Tick> {
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
            V3Pool::find_nearest_ticks(&self.contract, self.current_tick, self.tick_spacing).await;

        self.active_ticks = V3Pool::fetch_tick_data(&self.contract, &nearest_ticks).await;
    }
}
