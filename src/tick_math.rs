//! Utility functions for Uniswap V3 tick bitmap and tick index math
use crossbeam_channel::tick;
use ethers::types::U256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick {
    pub tick: i32,
    pub liquidityNet: i128,
}

/// Normalize a tick by tick spacing (division towards zero)
pub fn normalize_tick(current_tick: i32, tick_spacing: i32) -> i32 {
    current_tick.div_euclid(tick_spacing)
}

/// Calculate the word index in the bitmap for a normalized tick
pub fn word_index(normalized_tick: i32) -> i32 {
    normalized_tick.div_euclid(256)
}

/// Generate a range of word indices around the current index with given range
pub fn word_indices(center: i32, range: i32) -> Vec<i32> {
    ((center - range)..=(center + range)).collect()
}

/// Extract initialized tick values from a single bitmap word
pub fn extract_ticks_from_bitmap(bitmap: U256, word_idx: i32, tick_spacing: i32) -> Vec<i32> {
    let mut ticks = Vec::new();
    if bitmap.is_zero() {
        return ticks;
    }
    for bit in 0..256 {
        if bitmap.bit(bit) {
            let normalized = (word_idx * 256) + (bit as i32);
            ticks.push(normalized * tick_spacing);
        }
    }
    ticks
}

/// Given a map of word_index -> bitmap, produce all initialized ticks
pub fn collect_ticks_from_map(
    word_map: &std::collections::HashMap<i32, U256>,
    tick_spacing: i32,
) -> Vec<i32> {
    let mut ticks = Vec::new();
    for (&word_idx, &bitmap) in word_map.iter() {
        ticks.extend(extract_ticks_from_bitmap(bitmap, word_idx, tick_spacing));
    }
    ticks.sort_unstable();
    ticks
}

fn price_from_tick(target_tick: i32) -> Option<U256> {
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
/// Convert a sqrt price Q128.96 to the nearest tick index (i32)
/// Port of Uniswap V3's TickMath.getTickAtSqrtRatio
pub fn tick_from_price(sqrt_price_x96: U256) -> Option<i32> {
    // Define bounds as U256 to avoid u128 overflow
    let min_sqrt = U256::from(4295128739u64);
    let max_sqrt = U256::from_dec_str("1461446703485210103287273052203988822378723970342").unwrap();

    if sqrt_price_x96 < min_sqrt || sqrt_price_x96 >= max_sqrt {
        eprintln!("Sqrt price {} out of bounds", sqrt_price_x96);
        return None;
    }

    // Convert to Q128.128 for log calculation
    let ratio = sqrt_price_x96 << 32;

    // Compute log2(ratio)
    let msb = 255 - ratio.leading_zeros() as i32;
    let mut log2 = (U256::from(msb) - U256::from(128u8)) << 64;

    let mut r = ratio >> (msb - 127);
    for i in 0..64 {
        r = (r * r) >> 127;
        let f = r >> 128;
        log2 |= f << (63 - i);
        r >>= f.as_u32();
    }

    // Calculate candidate ticks
    let tick_low = ((log2 - U256::from_dec_str("3402992956809132418596140100660247210").unwrap())
        >> 128)
        .as_u32() as i32;
    let tick_high = ((log2 + U256::from_dec_str("291339464771989622907027621153398088495").unwrap())
        >> 128)
        .as_u32() as i32;

    // Choose nearest
    if tick_low == tick_high {
        Some(tick_low)
    } else if price_from_tick(tick_high).unwrap_or(U256::zero()) <= sqrt_price_x96 {
        Some(tick_high)
    } else {
        Some(tick_low)
    }
}
