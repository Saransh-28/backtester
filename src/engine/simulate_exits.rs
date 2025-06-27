// src/engine/simulate_exits.rs

use crate::engine::position::Position;

/// SL → TP → EXP.  All PnL & fees in $, slippage only shifts the fill price.
pub fn simulate_position_exits(
    positions: &mut [Position],
    timestamps: &[f64],  // for expiration
    high: &[f64],
    low: &[f64],
    close: &[f64],
    exit_fee_rate: f64,
    slippage_rate: f64,
) {
    let n = high.len();

    for pos in positions.iter_mut() {
        if pos.is_closed { continue; }

        for j in pos.entry_index..n {
            let hit_sl = match pos.position_type.as_str() {
                "long"  => low[j]  <= pos.sl,
                "short" => high[j] >= pos.sl,
                _       => false,
            };
            let hit_tp = match pos.position_type.as_str() {
                "long"  => high[j] >= pos.tp,
                "short" => low[j]  <= pos.tp,
                _       => false,
            };
            let expired = pos.expiration_time
                .map_or(false, |t_exp| timestamps[j] >= t_exp);

            if hit_sl || hit_tp || expired {
                // 1) Choose raw exit price
                let raw_exit = if hit_sl {
                    pos.sl
                } else if hit_tp {
                    pos.tp
                } else {
                    close[j]
                };

                // 2) Apply slippage to get reported fill price
                let exit_price = match pos.position_type.as_str() {
                    "long"  => raw_exit * (1.0 - slippage_rate),
                    "short" => raw_exit * (1.0 + slippage_rate),
                    _       => raw_exit,
                };
                let slippage_exit = (raw_exit - exit_price).abs();

                // 3) Fee on notional
                let fee_exit = pos.position_size * exit_price * exit_fee_rate;

                // 4) Record in the struct
                pos.exit_index     = Some(j);
                pos.exit_price     = Some(exit_price);
                pos.exit_condition = Some(if hit_sl {"SL"} else if hit_tp {"TP"} else {"EXP"}.into());
                pos.slippage_exit  = slippage_exit;
                pos.fee_exit       = fee_exit;
                pos.is_closed      = true;

                // 5) Compute net PnL in $
                //    PnL = (fill_exit - fill_entry) * units  -  (fee_entry + fee_exit)
                let gross_pnl = (exit_price - pos.entry_price)
                              * pos.position_size
                              * if pos.position_type=="long" { 1.0 } else {-1.0};
                let pnl       = gross_pnl - (pos.fee_entry + pos.fee_exit);

                // 6) Returns
                let absolute_return = (exit_price / pos.entry_price) - 1.0;
                let real_return     = if pos.entry_price * pos.position_size > 0.0 {
                    pnl / (pos.entry_price * pos.position_size)
                } else {
                    0.0
                };

                pos.absolute_return = Some(absolute_return);
                pos.real_return     = Some(real_return);
                pos.pnl             = Some(pnl);

                break;
            }
        }
    }
}
