use alloy_primitives::{U256, U512, aliases::U24};

use crate::v3_base::{
    states::{TradeState, TradeStep},
    tick_math::{price_from_tick, tick_from_price},
    ticks::Ticks,
    v3_state::V3State,
    x96price_math::{compute_price_from0, compute_price_from1},
};

use super::err::{MathError, TickError, TradeError};

pub fn retry(trade_state: TradeState, ticks: &Ticks) -> Result<TradeState, TradeError> {
    println!("retrying trade");
    trade_loop(trade_state, ticks)
}

pub fn trade(
    pool: &V3State,
    fee: &U24,
    amount_in: U256,
    from0: bool,
) -> Result<TradeState, TradeError> {
    let trade_state = trade_start(pool, fee, amount_in, from0)?;
    trade_loop(trade_state, &pool.ticks)?;

    // build Trade
    Ok(trade_state)
}
//////////////////////////////
pub fn trade_start(
    pool: &V3State,
    fee: &U24,
    amount_in: U256,
    from0: bool,
) -> Result<TradeState, TradeError> {
    let mut trade_state = TradeState {
        fee_amount: U256::ZERO,
        remaining: amount_in.clone(),
        amount_out: U256::ZERO,
        x96price: pool.x96price,
        liquidity: pool.liquidity,
        amount_in,
        tick: pool.tick,
        from0,
        step: TradeStep::default(),
    };
    let fee_amount = amount_in
        .checked_mul(U256::from(*fee))
        .ok_or(MathError::A(trade_state))?
        .checked_div(U256::from(1_000_000))
        .ok_or(MathError::A(trade_state))?;
    trade_state.remaining = amount_in
        .checked_sub(fee_amount)
        .ok_or(MathError::A(trade_state))?;

    trade_state.fee_amount = fee_amount;
    trade_state.x96price = pool.x96price;
    trade_state.tick = tick_from_price(pool.x96price).ok_or(MathError::A(trade_state))?;
    trade_state.liquidity = pool.liquidity;
    match pool.ticks.get_tick(trade_state.tick) {
        Ok(_) => return Ok(trade_state),
        Err(_) => {
            trade_state.step.next_tick.tick = trade_state.tick; //just to work with the error
            return Err(TickError::Overflow(trade_state).into());
        }
    }
}
pub fn step_start(trade_state: &mut TradeState, ticks: &Ticks) -> Result<(), TradeError> {
    trade_state.step.next_tick_index = match ticks.get_tick_index(trade_state.tick) {
        Ok(i) => {
            if trade_state.from0 {
                if i + 1 >= ticks.len() {
                    return Err(TickError::Overflow(*trade_state).into());
                } // No ticks above
                i + 1
            } else {
                if i == 0 {
                    return Err(TickError::Underflow(*trade_state).into());
                } // No ticks below
                i - 1
            }
        }
        Err(i) => {
            if trade_state.from0 {
                if i >= ticks.len() {
                    return Err(TickError::Overflow(*trade_state).into());
                } // No ticks above
                i
            } else {
                if i == 0 {
                    return Err(TickError::Underflow(*trade_state).into());
                } // No ticks below
                i - 1
            }
        }
    };

    trade_state.step.next_tick = *ticks
        .get(trade_state.step.next_tick_index)
        .expect("checked above");

    if trade_state.step.next_tick.liquidity_net.is_none() {
        return Err(TickError::Unavailable(*trade_state).into());
    }
    // calculate the next tick’s price
    trade_state.step.next_price =
        price_from_tick(trade_state.step.next_tick.tick).ok_or(MathError::A(*trade_state))?;

    // compute max amount possible to cross this tick
    trade_state.step.amount_possible = get_delta(
        trade_state.from0,
        trade_state.liquidity,
        trade_state.x96price,
        trade_state.step.next_price,
    )
    .ok_or(MathError::A(*trade_state))?;

    Ok(())
}

////////////////////////////////////
///
pub fn get_delta(
    from0: bool,
    liquidity: U256,
    current_price: U256,
    next_price: U256,
) -> Option<U256> {
    // cross entire tick
    let delta = if from0 {
        let diff = next_price.checked_sub(current_price)?;
        let (dif, liq) = (U512::from(diff), U512::from(liquidity));

        U256::from(liq.checked_mul(dif)?.checked_div(U512::ONE << 96)?)
    } else {
        let diff = current_price.checked_sub(next_price)?;
        let (dif, liq) = (U512::from(diff), U512::from(liquidity));

        let den = U512::from(current_price.checked_mul(next_price)?);
        let num = liq.checked_mul(dif)?;

        let div = num.checked_div(den)?;
        let r = div.checked_mul(U512::ONE << 96)?;

        U256::from(r)
    };

    Some(delta)
}

pub fn trade_loop(mut trade_state: TradeState, ticks: &Ticks) -> Result<TradeState, TradeError> {
    while trade_state.remaining > U256::ZERO {
        step_start(&mut trade_state, ticks)?;
        println!("hi");
        if trade_state.remaining < trade_state.step.amount_possible {
            println!("not enough to cross finishing it");
            handle_non_crossing_step(&mut trade_state)?;
            break;
        }

        println!("gettiing delta to cross");
        if let Some(delta) = get_delta(
            trade_state.from0,
            trade_state.liquidity,
            trade_state.x96price,
            trade_state.step.next_price,
        ) {
            trade_state.step.delta = delta;
        } else {
            return Err(TradeError::Math(MathError::A(trade_state)));
        }

        println!("prepping next step");

        update_state_for_next_step(&mut trade_state)?;
    }
    Ok(trade_state)
}

pub fn update_state_for_next_step(trade_state: &mut TradeState) -> Result<(), TradeError> {
    trade_state.amount_out = trade_state
        .amount_out
        .checked_add(trade_state.step.delta)
        .ok_or(MathError::A(*trade_state))?;
    // update liquidity
    if let Some(net) = trade_state.step.next_tick.liquidity_net {
        trade_state.liquidity = if trade_state.from0 {
            if net > 0 {
                trade_state.liquidity.saturating_add(U256::from(net))
            } else {
                trade_state.liquidity.saturating_sub(U256::from(-net))
            }
        } else {
            if net < 0 {
                trade_state.liquidity.saturating_add(U256::from(net))
            } else {
                trade_state.liquidity.saturating_sub(U256::from(net))
            }
        };
    }
    trade_state.tick = trade_state.step.next_tick.tick;
    trade_state.x96price = trade_state.step.next_price;
    trade_state.remaining = trade_state
        .remaining
        .checked_sub(trade_state.step.amount_possible)
        .ok_or(MathError::A(*trade_state))?;

    Ok(())
}
/////////////////////////////////////
pub fn handle_non_crossing_step(trade_state: &mut TradeState) -> Result<(), TradeError> {
    // won't cross full tick
    let new_price = if trade_state.from0 {
        compute_price_from0(
            &trade_state.remaining,
            &trade_state.liquidity,
            &trade_state.x96price,
            true,
        )
        .ok_or(MathError::A(*trade_state))? //deference &mut let it use copy
    } else {
        compute_price_from1(
            &trade_state.remaining,
            &trade_state.liquidity,
            &trade_state.x96price,
            true,
        )
        .ok_or(MathError::A(*trade_state))?
    };

    let delta = get_delta(
        trade_state.from0,
        trade_state.liquidity,
        trade_state.x96price,
        trade_state.step.next_price,
    )
    .unwrap();

    trade_state.step.delta = delta;

    println!("{}", trade_state.step.delta);

    println!("be");
    trade_state.amount_out = trade_state
        .amount_out
        .checked_add(U256::from(delta))
        .ok_or(MathError::A(*trade_state))?;
    trade_state.remaining = U256::ZERO;
    trade_state.x96price = U256::from(new_price);

    Ok(())
}
