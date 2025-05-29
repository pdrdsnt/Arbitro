use std::{collections::HashMap, str::FromStr};

use ethers::{
    abi::Address,
    contract::{Contract, ContractError},
    types::{H160, U256},
};
use ethers_providers::Provider;
use futures::future::join_all;

use crate::{
    mult_provider::MultiProvider,
    tick_math::{self, Tick},
    token::Token,
    trade::Trade,
};

#[derive(Debug)]
pub struct V3PoolSim {
    pub address: Address,
    pub token0: Token,
    pub token1: Token,
    pub exchange: String,
    pub version: String,
    pub fee: u32,
    pub active_ticks: Vec<Tick>,
    pub tick_spacing: i32,
    pub liquidity: U256,
    pub x96price: U256,
}

impl V3PoolSim {
    // Private constructor
    pub fn new(
        address: Address,
        fee: u32,
        dex: String,
        version: String,
        token0: Token,
        token1: Token,
        tick_spacing: i32,
        active_ticks: Vec<Tick>,
        liquidity: U256,
        x96price: U256,
    ) -> Self {
        Self {
            address,
            token0,
            token1,
            exchange: dex,
            version,
            active_ticks: active_ticks,
            fee,
            tick_spacing: tick_spacing,
            liquidity: liquidity,
            x96price: x96price,
        }
    }
    pub fn trade(&mut self, amount_in: U256, from0: bool) -> Option<Trade> {
        // 1. Fee deduction
        let fee_amount = amount_in
            .checked_mul(U256::from(self.fee))?
            .checked_div(U256::from(1_000_000))?;
        let mut remaining = amount_in.checked_sub(fee_amount)?;

        // 2. Local state
        let mut total_out = U256::zero();

        let mut curr_price = self.x96price;

        let mut current_tick = tick_math::tick_from_price(self.x96price)?;
        let mut next_tick_index = match self
            .active_ticks
            .binary_search_by_key(&current_tick, |t| t.tick)
        {
            Ok(i) => {
                if from0 {
                    if i + 1 >= self.active_ticks.len() {
                        return None;
                    } // No ticks above
                    i + 1
                } else {
                    if i == 0 {
                        return None;
                    } // No ticks below
                    i - 1
                }
            }
            Err(i) => {
                if from0 {
                    if i >= self.active_ticks.len() {
                        return None;
                    } // No ticks above
                    i
                } else {
                    if i == 0 {
                        return None;
                    } // No ticks below
                    i - 1
                }
            }
        };
        let mut curr_liq = self.liquidity;

        // 3. Iterate ticks
        while remaining > U256::zero() {
            // get target tick price
            let tick = self.active_ticks.get(next_tick_index as usize)?;
            let next_price = Self::tick_price(tick.tick)?;
            next_tick_index = if from0 {
                next_tick_index.checked_add(1)?
            } else {
                next_tick_index.checked_sub(1)?
            };

            // compute max amount possible to cross this tick
            let possible =
                Self::compute_amount_possible(from0, &curr_liq, &curr_price, &next_price)?;

            ////-------------------------------
            ///
            ////-------------------------------
            ///
            if remaining < possible {
                // won't cross full tick
                let new_price = if from0 {
                    Self::compute_price_from0(&remaining, &curr_liq, &curr_price, true)?
                } else {
                    Self::compute_price_from1(&remaining, &curr_liq, &curr_price, true)?
                };

                // compute out
                let delta = if from0 {
                    curr_liq
                        .checked_mul(new_price.checked_sub(curr_price)?)?
                        .checked_div(U256::from(1u128 << 96))?
                } else {
                    let inv_curr = (U256::one() << 96)
                        .checked_mul(U256::one() << 96)?
                        .checked_div(curr_price)?;
                    let inv_new = (U256::one() << 96)
                        .checked_mul(U256::one() << 96)?
                        .checked_div(new_price)?;
                    curr_liq
                        .checked_mul(inv_curr.checked_sub(inv_new)?)?
                        .checked_div(U256::from(1u128 << 96))?
                };

                total_out = total_out.checked_add(delta)?;
                remaining = U256::zero();
                curr_price = new_price;
                break;
            }

            // cross entire tick
            let out_cross = if from0 {
                curr_liq
                    .checked_mul(next_price.checked_sub(curr_price)?)?
                    .checked_div(U256::from(1u128 << 96))?
            } else {
                let num = curr_liq.checked_mul(curr_price.checked_sub(next_price)?)?;
                num.checked_div(U256::from(1u128 << 96))?
            };
            total_out = total_out.checked_add(out_cross)?;

            // update liquidity
            let net = tick.liquidityNet;
            curr_liq = if from0 {
                if net > 0 {
                    curr_liq.saturating_add(U256::from(net))
                } else {
                    curr_liq.saturating_sub(U256::from(-net))
                }
            } else {
                if net < 0 {
                    curr_liq.saturating_add(U256::from(net))
                } else {
                    curr_liq.saturating_sub(U256::from(net))
                }
            };

            // move pointer
            curr_price = next_price;
            remaining = remaining.checked_sub(possible)?;
        }

        self.liquidity = curr_liq;
        self.x96price = curr_price;

        // build Trade
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
            price_impact: fee_amount,
            fee_amount,
            raw_price: total_out,
        })
    }

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
        for (mask, magic) in magic_numbers.iter() {
            if abs_tick & mask != 0 {
                // wrap on overflow, then shift down
                let (wrapped, _) = ratio.overflowing_mul(*magic);
                ratio = wrapped >> 128;
            }
        }

        // Uniswap does: if tick > 0, invert the Q128.128 ratio
        if target_tick > 0 {
            // type(uint256).max / ratio in Solidity
            ratio = U256::MAX / ratio;
        }

        // Finally convert Q128.128 → Q128.96 by shifting 32 bits (rounding up)
        let sqrt_price_x96 = {
            let shifted = ratio >> 32;
            if ratio & ((U256::one() << 32) - U256::one()) != U256::zero() {
                shifted + U256::one()
            } else {
                shifted
            }
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
}
