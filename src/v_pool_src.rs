use std::str::FromStr;

use bigdecimal::{BigDecimal, ToPrimitive};
use ethers::types::{H160, U256};
use num_traits::Zero;

use crate::{tick_math::Tick, trade::Trade, v2_pool_src::V2PoolSrc, v3_pool_src::V3PoolSrc};
#[derive(Debug,)]
pub enum AnyPoolSrc {
    V2(V2PoolSrc,),
    V3(V3PoolSrc,),
}

impl AnyPoolSrc {
    pub async fn update(&mut self,) -> Result<H160, crate::err::PoolUpdateError,> {
        match self {
            AnyPoolSrc::V2(src,) => src.update().await,
            AnyPoolSrc::V3(src,) => src.update().await,
        }
    }

    pub async fn into_sim(&self,) -> crate::v_pool_sim::AnyPoolSim {
        match self {
            AnyPoolSrc::V2(v2_pool,) => crate::v_pool_sim::AnyPoolSim::V2(v2_pool.into_sim().await,),
            AnyPoolSrc::V3(v3_pool,) => crate::v_pool_sim::AnyPoolSim::V3(v3_pool.into_sim().await,),
        }
    }

    pub async fn get_tokens(&self,) -> [H160; 2] {
        match self {
            AnyPoolSrc::V2(v2_pool,) => [v2_pool.token0.read().await.address, v2_pool.token1.read().await.address,],
            AnyPoolSrc::V3(v3_pool,) => [v3_pool.token0.read().await.address, v3_pool.token1.read().await.address,],
        }
    }
    /// Is the given address the token0 of this pool?
    pub async fn is_0(&self, addr: &H160,) -> bool {
        match self {
            AnyPoolSrc::V2(v2,) => v2.token0.read().await.address == *addr,
            AnyPoolSrc::V3(v3,) => v3.token0.read().await.address == *addr,
        }
    }
    pub async fn in_pool(&self, addr: H160,) -> bool {
        match self {
            AnyPoolSrc::V2(v2_pool,) => v2_pool.token0.read().await.address == addr,
            AnyPoolSrc::V3(v3_pool,) => v3_pool.token0.read().await.address == addr,
        }
    }
    pub fn get_address(&self,) -> H160 {
        match self {
            AnyPoolSrc::V2(v2_pool,) => v2_pool.address,
            AnyPoolSrc::V3(v3_pool,) => v3_pool.address,
        }
    }
    pub fn get_fee(&self,) -> u32 {
        match self {
            AnyPoolSrc::V2(v2_pool,) => v2_pool.fee,
            AnyPoolSrc::V3(v3_pool,) => v3_pool.fee,
        }
    }
    pub fn get_version(&self,) -> String {
        match self {
            AnyPoolSrc::V2(_,) => "v2".to_string(),
            AnyPoolSrc::V3(_,) => "v3".to_string(),
        }
    }
    pub fn get_dex(&self,) -> String {
        match self {
            AnyPoolSrc::V2(v2_pool,) => v2_pool.exchange.clone(),
            AnyPoolSrc::V3(v3_pool,) => v3_pool.exchange.clone(),
        }
    }
    pub fn get_price(&self, decimals: [u8; 2],) -> Option<f64,> {
        match self {
            AnyPoolSrc::V2(v2_pool,) => {
                println!("V2 pool price calculation");
                println!("Reserves: {:?}", v2_pool.reserves0);
                println!("Reserves: {:?}", v2_pool.reserves1);
                let raw_price = Self::get_v2_raw_price(&v2_pool.reserves0, &v2_pool.reserves1,)?;
                Self::adjust_for_decimals(raw_price, decimals,)
            },
            AnyPoolSrc::V3(v3_pool,) => {
                let raw_price = Self::get_v3_raw_price(v3_pool.x96price,)?;
                Self::adjust_for_decimals(raw_price, decimals,)
            },
        }
    }
    // V2-specific price calculation
    fn get_v2_raw_price(reserves0: &U256, reserves1: &U256,) -> Option<BigDecimal,> {
        let r0 = Self::big_decimal_from_u256(reserves0,)?;
        let r1 = Self::big_decimal_from_u256(reserves1,)?;

        if r0 == BigDecimal::zero() {
            return None;
        }

        Some(r1 / r0,)
    }
    // V3-specific price calculation
    fn get_v3_raw_price(price_x96: U256,) -> Option<BigDecimal,> {
        if price_x96.is_zero() {
            return None;
        }

        let numerator = Self::big_decimal_from_u256(&price_x96,)?;
        let denominator = Self::big_decimal_from_u128(1u128 << 96,)?;

        Some(numerator / denominator,)
    }
    // Decimal adjustment logic
    pub fn adjust_for_decimals(raw_price: BigDecimal, decimals: [u8; 2],) -> Option<f64,> {
        let exp = (decimals[1] as i32) - (decimals[0] as i32);
        let abs_exp = exp.abs() as u32;
        let factor = 10u64.pow(abs_exp,);

        let adjusted_price = if exp >= 0 {
            raw_price * BigDecimal::from(factor,)
        } else {
            raw_price / BigDecimal::from(factor,)
        };

        adjusted_price.to_f64().or(Some(0.0,),)
    }
    // Helper function for safe U256 to BigDecimal conversion
    fn big_decimal_from_u256(value: &U256,) -> Option<BigDecimal,> { BigDecimal::from_str(&value.to_string(),).ok() }
    // Helper function for safe u128 to BigDecimal conversion
    fn big_decimal_from_u128(value: u128,) -> Option<BigDecimal,> { BigDecimal::from_str(&value.to_string(),).ok() }

    pub fn get_reserves(&self,) -> Option<(U256, U256,),> {
        match self {
            AnyPoolSrc::V2(v2_pool,) => {
                // Assuming V2Pool has an async get_reserves() -> (U256, U256)
                Some((v2_pool.reserves0, v2_pool.reserves1,),)
            },
            AnyPoolSrc::V3(v3_pool,) => {
                // Assuming V3Pool has an async get_reserves() -> (U256, U256)
                let Q96 = 2u128.pow(96,);
                let liquidity = v3_pool.liquidity;
                let sqrt_price_x96 = v3_pool.x96price;
                // 2) reserve0 = liquidity * 2^96 / sqrtPriceX96
                let numerator0 = liquidity.saturating_mul(U256::one() << 96,);
                let r0 = numerator0.checked_div(sqrt_price_x96,).unwrap_or(U256::zero(),);

                // 3) reserve1 = liquidity * sqrtPriceX96 / 2^96
                let r1 = liquidity.saturating_mul(sqrt_price_x96,) >> 96;

                Some((r0, r1,),)
            },
        }
    }

    pub fn active_ticks(&self,) -> Option<&Vec<Tick,>,> {
        match self {
            AnyPoolSrc::V3(v3,) => Some(&v3.active_ticks,),
            _ => None,
        }
    }

    /// Returns the V3 pool’s tick spacing, or `None` if this is a V2 pool.
    pub fn tick_spacing(&self,) -> Option<i32,> {
        match self {
            AnyPoolSrc::V3(v3,) => Some(v3.tick_spacing,),
            _ => None,
        }
    }

    /// Returns the V3 pool’s current liquidity, or `None` if this is a V2 pool.
    pub fn liquidity(&self,) -> Option<&U256,> {
        match self {
            AnyPoolSrc::V3(v3,) => Some(&v3.liquidity,),
            _ => None,
        }
    }

    /// Returns the V3 pool’s sqrt-price (x96), or `None` if this is a V2 pool.
    pub fn x96_price(&self,) -> Option<&U256,> {
        match self {
            AnyPoolSrc::V3(v3,) => Some(&v3.x96price,),
            _ => None,
        }
    }
}
