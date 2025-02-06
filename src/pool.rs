use crate::{pool_utils::{Tick, Trade}, token::Token};
use bigdecimal::{self, BigDecimal, FromPrimitive};
use std::str::FromStr;

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
pub trait Pool {
    async fn update(&mut self);
    fn trade(&self, amount_in: u32, from: bool) -> Trade;
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
        address: Address,
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: " ".to_string(),
            version: "v2".to_string(),
            fee: 3000,
            reserves0: U256::from(0),
            reserves1: U256::from(0),
            contract,
        }
    }

    pub async fn new_with_update(
        address: Address,
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = V2Pool::new(address, token0, token1, contract).await;
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

    fn trade(&self, amount_in: u32, from0: bool) -> Trade {
        let big_amount_in = BigDecimal::from_u32(amount_in).unwrap();
        let r0 = BigDecimal::from_str(&self.reserves0.to_string()).unwrap();
        let r1 = BigDecimal::from_str(&self.reserves1.to_string()).unwrap();

        // Calculate the input and output reserves based on the trade direction
        let (reserve_in, reserve_out) = if from0 { (&r0, &r1) } else { (&r1, &r0) };

        // Fee multiplier: assume self.fee is in basis points (e.g., 30 for 0.3%)
        let fee_multiplier =
            BigDecimal::from(1) - (BigDecimal::from(self.fee) / BigDecimal::from(10_000));

        // Adjust the input amount for the fee
        let amount_in_with_fee = &big_amount_in * &fee_multiplier;

        // Calculate amount out using the constant product formula
        // amount_out = (reserve_out * amount_in_with_fee) / (reserve_in + amount_in_with_fee)
        let numerator = reserve_out * &amount_in_with_fee;
        let denominator = reserve_in + &amount_in_with_fee;
        let amount_out = numerator / denominator;

        // New reserves after the swap
        let (new_r0, new_r1) = if from0 {
            (&r0 + &big_amount_in, &r1 - &amount_out)
        } else {
            (&r0 - &amount_out, &r1 + &big_amount_in)
        };

        // Calculate current and new prices
        let current_price = if from0 { &r1 / &r0 } else { &r0 / &r1 };
        let new_price = if from0 {
            &new_r1 / &new_r0
        } else {
            &new_r0 / &new_r1
        };

        // Price impact
        let price_impact = (&current_price - &new_price) / &current_price;

        // Create the trade data object
        Trade {
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in: big_amount_in.clone(),
            amount_out,
            price_impact,
            fee: &big_amount_in - &amount_in_with_fee, // Fee amount
            raw_price: current_price,
        }
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
        token0: Token,
        token1: Token,
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: " ".to_string(),
            version: "v2".to_string(),
            current_tick: 0,
            active_ticks: Vec::new(),
            fee: 3000,
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
        contract: Contract<Provider<Ws>>,
    ) -> Self {
        let mut instance = Self::new(address, token0, token1, contract).await;

        let fee_call = instance.contract.method::<(), (u32)>("fee", ());
        let fee_call_result = fee_call.unwrap().call_raw().await;
        let fee = fee_call_result.unwrap();
        instance.fee = fee;

        let spacing_call = instance.contract.method::<(), (i32)>("tickSpacing", ());
        let spacing_call_result = spacing_call.unwrap().call_raw().await;
        let spacing = spacing_call_result.unwrap();
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

        let liquidity_call_result = self.contract.method::<(), (U256)>("slot0", ()); // only U256 implements detokenizable and need to be converted

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

        let previous_ticks = Vec::<Tick>::new();

        let normalized_tick = self.current_tick / self.tick_spacing;
        let current_word_idx: i16 = (normalized_tick / 256) as i16;
        let current_bit_idx: i16 = (normalized_tick % 256) as i16;

        let mut current_word: U256 = U256::zero();
        let mut next_word: U256 = U256::zero();
        let mut previous_word: U256 = U256::zero();

        async fn fetch_bitmap(
            contract: &Contract<Provider<Ws>>,
            index: i16,
        ) -> Result<U256, ContractError<Provider<Ws>>> {
            let call = contract.method::<(i16), (U256)>("tickBitmap", (index))?;
            call.call_raw().await
        }

        if let Ok(word) = fetch_bitmap(&self.contract, current_word_idx).await {
            current_word = word;
        }
        if let Ok(word) = fetch_bitmap(&self.contract, current_word_idx + 1).await {
            next_word = word;
        }
        if let Ok(word) = fetch_bitmap(&self.contract, current_word_idx - 1).await {
            previous_word = word;
        }

        // Backward search (3 ticks)
        let mut previous_ticks = Vec::new();
        let mut mask_idx: i16 = current_bit_idx;
        let mut word = current_word;
        let mut changed_word = false;
        let mut found = 0;

        while found < 3 {
            while (U256::from(1) << mask_idx) & word == U256::zero() {
                if mask_idx == 0 {
                    if changed_word {
                        panic!("not enough words");
                    }
                    word = previous_word;
                    mask_idx = 255;
                    changed_word = true;
                } else {
                    mask_idx -= 1;
                }
            }
            // Compute tick based on the word index.
            let tick = ((current_word_idx - if changed_word { 1 } else { 0 }) as i32 * 256
                + mask_idx as i32)
                * self.tick_spacing;
            previous_ticks.push(tick);
            found += 1;
            mask_idx -= 1; // Continue searching
        }

        // Because the backward search collects ticks in descending order, reverse it:
        // no do not reverse it because when this direction will be used it will need to be inverted anyway
        // previous_ticks.reverse();

        // Forward search (3 ticks)
        let mut next_ticks = Vec::new();
        mask_idx = current_bit_idx;
        word = current_word;
        changed_word = false;
        found = 0;

        while found < 3 {
            while (U256::from(1) << mask_idx) & word == U256::zero() {
                if mask_idx == 255 {
                    if changed_word {
                        panic!("not enough words");
                    }
                    word = next_word;
                    mask_idx = 0;
                    changed_word = true;
                } else {
                    mask_idx += 1;
                }
            }
            let tick = ((current_word_idx - if changed_word { 1 } else { 0 }) as i32 * 256
                + mask_idx as i32)
                * self.tick_spacing;
            next_ticks.push(tick);
            found += 1;
            mask_idx += 1; // Continue searching
        }

        // Combine the ticks so that they are in ascending order:
        // Since previous_ticks is now in ascending order (lowest first)
        // and next_ticks is already in ascending order, just chain them.
        let sorted_ticks: Vec<i32> = previous_ticks
            .into_iter()
            .chain(next_ticks.into_iter())
            .collect();

        // Now call `ticks` for each tick found and store liquidityNet, preserving the order:
        let mut active_ticks = Vec::new();
        for tick in sorted_ticks {
            if let Ok(call) = self
                .contract
                .method::<(i32), (i128, i128, U256, U256, i64)>("ticks", tick)
            {
                if let Ok((liquidity_net, _, _, _, _)) = call.call_raw().await {
                    active_ticks.push(Tick {
                        tick,
                        liquidityNet: liquidity_net,
                    });
                }
            }
        }

        self.active_ticks = active_ticks;
    }
    fn trade(&self, amount_in: u32, from0: bool) -> Trade {
        // For now we assume we're swapping token0 for token1 when from0 == true.
        // (You would add a branch for the reverse case.)

        let ticks: &[Tick] = if from0 {
            self.active_ticks.split_at(2).1
        } else {
            self.active_ticks.split_at(2).0
        };

        let mut remaining_in = BigDecimal::from(amount_in);
        let mut total_out = BigDecimal::from(0);

        // Save the initial price for price impact calculation.
        let x96 = BigDecimal::from_str("79228162514264337593543950336").unwrap();
        let sqrt_price_bd = BigDecimal::from_str(&self.x96price.to_string()).unwrap();
        let initial_sqrt_price = &sqrt_price_bd / &x96;
        let initial_price = &initial_sqrt_price * &initial_sqrt_price;

        // Set starting state.
        let mut current_liquidity = self.liquidity;
        let mut current_tick = self.current_tick;
        let mut current_sqrt_price = initial_sqrt_price.clone();
        let mut current_price = initial_price.clone();

        // Apply fee to input amount.
        let fee_multiplier =
            BigDecimal::from(1) - BigDecimal::from(self.fee) / BigDecimal::from(1_000_000);
        remaining_in = remaining_in * &fee_multiplier;

        // Iterate over active ticks.
        // We assume self.active_ticks are sorted in the direction of the swap.
        for tick in ticks {
            // Calculate the next tick's sqrt price.
            // Uniswap V3 defines: sqrtPrice = 1.0001^(tick/2) * 2^96.
            // In relative terms, the ratio between ticks is 1.0001^((tick.tick - current_tick)/2).
            let base = BigDecimal::from_str("1.0001").unwrap();
            //dividing exponent by two to avoid srqroot
            let exponent = BigDecimal::from(tick.tick - current_tick) / BigDecimal::from(2);
            let ratio = bd_pow(&base, &exponent);
            let next_sqrt_price = &current_sqrt_price * &ratio;
            let next_price = &next_sqrt_price * &next_sqrt_price;

            // Compute the maximum input that can be absorbed in the current range before hitting the next tick.
            // For a token0 swap, the exact formula is non-linear:
            //    amount_in = liquidity * (new_sqrt - current_sqrt) / (new_sqrt * current_sqrt)
            // Here we compute the amount needed to reach next_sqrt_price.
            let available_liquidity = BigDecimal::from_str(&current_liquidity.to_string()).unwrap();
            let amount_possible = if from0 {
                (&available_liquidity * (&next_sqrt_price - &current_sqrt_price))
                    / (&next_sqrt_price * &current_sqrt_price)
            } else {
                &available_liquidity * (&current_sqrt_price - &next_sqrt_price)
            };

            if remaining_in < amount_possible {
                // The remaining amount does not cross the tick boundary.
                // Use a linear approximation to compute the output.
                let amount_out = (&remaining_in * &current_sqrt_price * &current_sqrt_price)
                    / &available_liquidity;
                total_out += amount_out;
                remaining_in = BigDecimal::from(0);
                break;
            } else {
                // Consume the entire range up to next tick.
                total_out += (&amount_possible * &current_sqrt_price * &current_sqrt_price)
                    / &available_liquidity;
                remaining_in -= &amount_possible;

                // Cross the tick: update current_tick and liquidity.
                current_tick = tick.tick;
                // liquidityNet may be negative; here we convert via BigDecimal.
                let liquidity_bd = BigDecimal::from_str(&current_liquidity.to_string()).unwrap();
                let updated_liquidity_bd = liquidity_bd + BigDecimal::from(tick.liquidityNet);
                // For simplicity, assume liquidity remains non-negative.
                let updated_liquidity: u128 = updated_liquidity_bd.to_string().parse().unwrap();
                current_liquidity = U256::from(updated_liquidity);

                // Update price to next tick.
                current_sqrt_price = next_sqrt_price;
                current_price = next_price;
            }
        }

        let decimal_diff = if from0 {
            self.token0.decimals as i32 - self.token1.decimals as i32
        } else {
            self.token1.decimals as i32 - self.token0.decimals as i32
        };
    
        let scaling_factor = bd_pow(&BigDecimal::from(10),&BigDecimal::from(decimal_diff));
        let adjusted_total_out = &total_out * scaling_factor;
    

        // Calculate price impact relative to the initial price.
        let price_impact = (&current_price - &initial_price) / &initial_price;

        Trade {
            token0: self.token0.address,
            token1: self.token1.address,
            pool: self.address,
            from0,
            amount_in: BigDecimal::from(amount_in),
            amount_out: total_out,
            price_impact,
            fee: BigDecimal::from(amount_in) - remaining_in, // Total fee deducted.
            raw_price: current_price,
        }
    }
}

// A simple helper function for exponentiation of BigDecimal using f64 conversion.
// Note: This loses precision and should be replaced with a more robust method if needed.
fn bd_pow(base: &BigDecimal, exponent: &BigDecimal) -> BigDecimal {
    let base_f64: f64 = base.to_string().parse().unwrap();
    let exp_f64: f64 = exponent.to_string().parse().unwrap();
    let result_f64 = base_f64.powf(exp_f64);
    BigDecimal::from_f64(result_f64).unwrap()
}
