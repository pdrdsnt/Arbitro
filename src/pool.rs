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
        consts::{U16, U160, U24, U25, U8},
    },
    providers::{Http, Provider, Ws},
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
    pub contract: Contract<Provider<Ws>>,
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
        contract: Contract<Provider<Ws>>,
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
        contract: Contract<Provider<Ws>>,
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
            (r0 + big_amount_in, r1 - amount_out)
        } else {
            (r0 - amount_out, r1 + big_amount_in)
        };

        // Calculate current and new prices
        let current_price = if from0 {
            if *r0 == U256::from(0) {
                return None;
            }
            r1 / r0
        } else {
            if *r1 == U256::from(0) {
                return None;
            }
            r0 / r1
        };
        let new_price = if from0 {
            if new_r0 == U256::from(0) {
                return None;
            }
            new_r1 / new_r0
        } else {
            if new_r1 == U256::from(0) {
                return None;
            }
            new_r0 / new_r1
        };

        // Price impact
        let price_impact = if current_price > U256::from(0) {
            (&current_price - &new_price) / &current_price
        } else {
            U256::from(0)
        };
        // Create the trade data object
        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in: big_amount_in.clone(),
            amount_out,
            price_impact,
            fee: &big_amount_in - &amount_after_fee, // Fee amount
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
    pub contract: Contract<Provider<Ws>>,
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
        contract: Contract<Provider<Ws>>,
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
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = Self::new(address, fee, dex, version, token0, token1, contract).await;

        instance.fee = fee;

        let spacing_call = instance.contract.method::<(), i32>("tickSpacing", ());
        let spacing_call_result = spacing_call.unwrap().call_raw().await;
        let spacing = match spacing_call_result {
            Ok(spacing) => spacing,
            Err(erro) => {
                println!("abi erro {}", erro);
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
            .method::<(), (U256, i32, U256, U256, U256, U256, bool)>("slot0", ()); // only U256 implements detokenizable and need to be converted
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
                    Err(erro) => println!("contract call error {}", erro),
                }
            }
            Err(erro) => println!("abi erro {}", erro),
        }

        let liquidity_call_result = self.contract.method::<(), (U256)>("liquidity", ()); // only U256 implements detokenizable and need to be converted

        match liquidity_call_result {
            Ok(liquidity) => {
                let var = liquidity.call_raw().await;

                match var {
                    Ok(liquidity) => {
                        self.liquidity = liquidity;
                    }
                    Err(erro) => println!("contract call error {}", erro),
                }
            }
            Err(erro) => println!("abi erro {}", erro),
        }

        V3Pool::update_active_ticks(self).await;
    }

    fn trade(&self, amount_in: U256, from0: bool) -> Option<Trade> {
        let mut ticks = self.active_ticks.clone();
        // Filter ticks based on the current tick and direction of the swap.
        if from0 {
            ticks.retain(|&t| t.tick > self.current_tick); // ascending order
        } else {
            ticks.retain(|&t| t.tick < self.current_tick);
            ticks.sort_by(|a, b| b.cmp(a)); // descending order
        };

        let mut remaining_in = U256::from(amount_in);
        let mut total_out = U256::from(0);

        // Save the initial price for price impact calculation.
        let x96 = U256::from_str("79228162514264337593543950336").unwrap();
        let sqrt_price_bd = &self.x96price.clone();
        let initial_sqrt_price = &sqrt_price_bd;

        // Set starting state.
        let mut current_liquidity = self.liquidity;
        let mut current_tick: i32 = self.current_tick;
        let mut current_sqrt_price = *initial_sqrt_price.clone();

        let fee_percent = U256::from(self.fee);
        let fee_amount = amount_in
            .checked_mul(fee_percent)
            .and_then(|v| v.checked_div(U256::from(1_000_000)))?;

        // Apply fee to input amount (0.3% = 3000/1e6)
        let mut remaining_in = amount_in.checked_sub(fee_amount)?;

        // Precompute constants for tick math
        let TICK_BASE_NUMERATOR: U256 = U256::from(10001); // 1.0001 scaled by 1e4
        let TICK_BASE_DENOMINATOR: U256 = U256::from(10000);
        let Q96: U256 = U256::from(1) << 96; // 2^96 in Q64.96 format


        // Iterate over active ticks.
        // We assume self.active_ticks are sorted in the direction of the swap.
        for tick in ticks {
            // Calculate the next tick's sqrt price.
            let next_sqroot_price = V3Pool::tick_price(tick.tick)?;
            // Uniswap V3 defines: sqrtPrice = 1.0001^(tick/2) * 2^96.
            // In relative terms, the ratio between ticks is 1.0001^((tick.tick - current_tick)/2).
           
            // Compute the maximum input that can be absorbed in the current range before hitting the next tick.
            // For a token0 swap, the exact formula is non-linear:
            //    amount_in = liquidity * (new_sqrt - current_sqrt) / (new_sqrt * current_sqrt)
            // Here we compute the amount needed to reach next_sqrt_price.
            let available_liquidity = current_liquidity.clone();
            let mut ticks = self.active_ticks.clone();

            let amount_possible = V3Pool::compute_amount_possible(
                from0,
                &available_liquidity,
                &current_sqrt_price,
                &next_sqroot_price,
            )?;

            if remaining_in < amount_possible {
                // The remaining amount does not cross the tick boundary.
                // Use a linear approximation to compute the output.
                let amount_out = if from0 {
                    // Δy = Δx * L / (Δx + L / P)
                    V3Pool::compute_swap_amount_from0(
                        &remaining_in,
                        &available_liquidity,
                        &current_sqrt_price,
                        &next_sqroot_price,
                    )?
                } else {
                    V3Pool::compute_swap_amount_from1(
                        &remaining_in,
                        &available_liquidity,
                        &current_sqrt_price,
                    )?
                };
                total_out += amount_out;
                println!("reamining in in bounds {}", remaining_in);
                remaining_in = U256::from(0);
                break;
            } else {
                // Consume the entire range up to next tick.
                let delta_out = available_liquidity
                .checked_mul(next_sqroot_price.checked_sub(current_sqrt_price)?)?;
                total_out = total_out.checked_add(delta_out)?;



                // Cross the tick: update current_tick and liquidity.
                current_tick = tick.tick;
               
                let liquidity_net = tick.liquidityNet;
                if liquidity_net >= 0 {
                    current_liquidity += U256::from(liquidity_net as u128);
                } else {
                    let abs_net = (-liquidity_net) as u128;
                    current_liquidity = current_liquidity.saturating_sub(U256::from(abs_net));
                }

                // Update price to next tick.
                current_sqrt_price = next_sqroot_price;
            }
        }
    
        // NEW: Handle precision dust (rounding errors)
        if remaining_in < U256::from(10) {
            // Threshold for acceptable dust
            remaining_in = U256::zero();
        }

        if remaining_in > U256::zero() {
            println!("Remaining amount in: {}", remaining_in);
            println!("not enough liquidity");
            return None;
        }

        // Create the trade data object

        println!("trade simulated in dex {} {}", self.exchange, self.version);
        println!("amount_in {}", amount_in);
        println!("amount_out {}", total_out);
        println!("from {} to {}", self.token0.symbol, self.token1.symbol);
        Some(Trade {
            dex: self.exchange.clone(),
            version: self.version.clone(),
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in: amount_in,
            amount_out: total_out,
            price_impact: amount_in,
            fee: fee_amount, // Total fee deducted.
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
        if from0 {
            // For token0, calculate the difference as (next - current)
            let diff = next_sqrt_price.checked_sub(*current_sqrt_price)?;
            if diff == U256::zero() {
                return None;
            }
            // amount_possible = (available_liquidity * diff) / (next_sqrt_price * current_sqrt_price)
            Some(available_liquidity * diff / (next_sqrt_price * current_sqrt_price))
        } else {
            // For token1, calculate the difference as (current - next)
            let diff = current_sqrt_price.checked_sub(*next_sqrt_price)?;
            if diff == U256::zero() {
                return None;
            }
            // amount_possible = available_liquidity * diff
            Some(available_liquidity * diff)
        }
    }

    fn compute_swap_amount_from0(
        amount: &U256,
        available_liquidity: &U256,
        current_sqrt_price: &U256,
        next_sqrt_price: &U256,
    ) -> Option<U256> {
        // For token0, calculate the difference as (next - current)
        let n = amount
        .checked_mul(*current_sqrt_price)?
        .checked_mul(*next_sqrt_price)?;

        let d = n.checked_div(*available_liquidity)?;

        Some(current_sqrt_price.checked_add(d)?)
    }

    fn compute_swap_amount_from1(
        amount: &U256,
        available_liquidity: &U256,
        current_sqrt_price: &U256,
    ) -> Option<U256> {
        // For token1, calculate the difference as (current - next)
        let n = amount.checked_div(*available_liquidity)?;
        // amount_possible = available_liquidity * diff
        Some(current_sqrt_price.checked_sub(n)?)
    }

    fn tick_price(target_tick: i32) -> Option<U256> {
        // Calculate 1.0001^tick using integer math
        let abs_tick = target_tick.unsigned_abs();
        let mut ratio = U256::from(1);

        // 1.0001 in Q128 format (1.0001 * 2^128)
        // Calculate 1.0001 in Q128 format using safe operations
        let base = (U256::from(10001u64) << 128) / U256::from(10000u64);

        // Use exponentiation by squaring
        let mut n = abs_tick;
        let mut current = base;
        while n > 0 {
            if n % 2 == 1 {
                // Multiply by base only when needed
                ratio = ratio.checked_mul(current)?;
            }
            //sqr base
            current = current.checked_mul(current)?;
            // Divide by 2 exponent
            n /= 2;
        }

        // Adjust for negative ticks
        if target_tick < 0 {
            ratio = U256::MAX / ratio;
        }

        // Calculate square root using Babylonian method
        let mut sqrt_ratio = ratio;
        let mut x = ratio;
        //first step
        let mut y = (x + U256::one()) >> 1;
        while y < x {
            x = y;
            y = (x + ratio / x) >> 1;
        }
        sqrt_ratio = x;

        // Convert to Q64.96 format (sqrt(ratio) * 2^96)
        sqrt_ratio.checked_mul(U256::from(2u128.pow(96)))
    }

    fn update_liquidity(current: U256, net: i128) -> Option<U256> {
        let _net = U256::from(net);

        if net > i128::from(0) {
            current.checked_sub(_net)
        } else {
            current.checked_sub(_net)
        }
    }

    async fn fetch_bitmap_words(
        contract: &Contract<Provider<Ws>>,
        word_indices: &[i32],
    ) -> HashMap<i32, U256> {
        let futures = word_indices.iter().map(|&idx| async move {
            match Self::fetch_bitmap_word(contract, idx as i16).await {
                Ok(word) => Some((idx, word)), // Store the result if successful
                Err(_) => {
                    println!("Failed to fetch word for index {}", idx);
                    None
                }
            }
        });

        let results = join_all(futures).await;
        results.into_iter().flatten().collect() // Convert `Option` to `HashMap`
    }

    /// Fetch a single bitmap word from the contract.
    async fn fetch_bitmap_word(
        contract: &Contract<Provider<Ws>>,
        index: i16,
    ) -> Result<U256, ContractError<Provider<Ws>>> {
        contract
            .method::<(i16), U256>("tickBitmap", (index))?
            .call()
            .await
    }

    /// Find the nearest initialized ticks around the current tick.
    async fn find_nearest_ticks(
        contract: &Contract<Provider<Ws>>,
        current_tick: i32,
        tick_spacing: i32,
        max_ticks: usize,
    ) -> Vec<i32> {
        let normalized_tick = current_tick.div_euclid(tick_spacing);
        let current_word_idx = normalized_tick.div_euclid(256);
        let current_bit_idx = normalized_tick.rem_euclid(256);

        let search_radius = ((max_ticks as f32 / 256.0).ceil() as i32).max(1);
        let word_indices: Vec<i32> = (-search_radius..=search_radius)
            .map(|offset| current_word_idx + offset)
            .collect();

        let word_map = Self::fetch_bitmap_words(contract, &word_indices).await;

        let mut ticks = Vec::with_capacity(max_ticks);
        let mut checked_bits = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((current_word_idx, current_bit_idx as i32));

        while let Some((word_idx, bit_idx)) = queue.pop_front() {
            if ticks.len() >= max_ticks {
                break;
            }
            if !checked_bits.insert((word_idx, bit_idx)) {
                continue;
            }

            println!("Checking word index: {}, bit index: {}", word_idx, bit_idx);

            if let Some(word) = word_map.get(&word_idx) {
                if word.bitand(U256::from(1) << bit_idx) != U256::zero() {
                    let tick = (word_idx * 256 + bit_idx) * tick_spacing;
                    ticks.push(tick);
                    println!("Found tick: {}", tick);
                }
            }

            // Expand within the same word
            if bit_idx > 0 && checked_bits.insert((word_idx, bit_idx - 1)) {
                queue.push_back((word_idx, bit_idx - 1));
            }
            if bit_idx < 255 && checked_bits.insert((word_idx, bit_idx + 1)) {
                queue.push_back((word_idx, bit_idx + 1));
            }

            // Expand to adjacent words if needed
            if bit_idx == 0 {
                if let Some(_) = word_map.get(&(word_idx - 1)) {
                    queue.push_back((word_idx - 1, 255));
                }
            }
            if bit_idx == 255 {
                if let Some(_) = word_map.get(&(word_idx + 1)) {
                    queue.push_back((word_idx + 1, 0));
                }
            }
        }

        ticks.sort();
        ticks.truncate(max_ticks);
        ticks
    }

    /// Fetch tick data for a given list of ticks.
    async fn fetch_tick_data(contract: &Contract<Provider<Ws>>, ticks: &[i32]) -> Vec<Tick> {
        let mut active_ticks = Vec::new();

        for &tick in ticks {
            match contract.method::<_, (i128, i128, U256, U256, i64)>("ticks", tick) {
                Ok(call) => match call.call().await {
                    Ok((liquidity_net, _, _, _, _)) => {
                        active_ticks.push(Tick {
                            tick,
                            liquidityNet: liquidity_net,
                        });
                    }
                    Err(err) => {
                        println!("Error fetching tick data for {}: {}", tick, err);
                    }
                },
                Err(err) => {
                    println!("Method call setup failed for tick {}: {}", tick, err);
                }
            }
        }
        active_ticks
    }

    async fn update_active_ticks(&mut self) {
        let nearest_ticks = V3Pool::find_nearest_ticks(
            &self.contract,
            self.current_tick,
            self.tick_spacing,
            10, // Find 6 nearest ticks (3 prev, 3 next).
        )
        .await;

        self.active_ticks = V3Pool::fetch_tick_data(&self.contract, &nearest_ticks).await;
    }
}
 