// src/engine/scan_entries.rs

use crate::engine::position::Position;

/// For each signal on bar i:
///  - we fill at bar i+1 open (or i if it's the last bar)
///  - we panic if both long[i] and short[i] are true
///  - expiration_times is aligned to the *signal* bar (i)
pub fn scan_entries(
    timestamps: &[f64],
    open: &[f64],
    long: &[bool],
    short: &[bool],
    long_tp: &[f64],
    long_sl: &[f64],
    short_tp: &[f64],
    short_sl: &[f64],
    long_size: &[f64],
    short_size: &[f64],
    expiration_times: &[f64],
    entry_fee_rate: f64,
    slippage_rate: f64,
) -> Vec<Position> {
    let n = open.len();

    // 1) Mutual-exclusion check + count total signals
    let mut total_signals = 0;
    for i in 0..n {
        if long[i] && short[i] {
            panic!("Signal conflict at bar {}: both long and short are true", i);
        }
        if long[i] || short[i] {
            total_signals += 1;
        }
    }

    // 2) Reserve capacity up-front
    let mut positions = Vec::with_capacity(total_signals);

    // 3) Build Position structs
    for i in 0..n {
        if !(long[i] || short[i]) {
            continue;
        }

        // fill bar
        let entry_idx = if i + 1 < n { i + 1 } else { i };
        let entry_ts  = timestamps[entry_idx];
        let price     = open[entry_idx];

        // expiration is aligned to the *signal* bar
        let exp_time = expiration_times.get(i).copied();
        if let Some(et) = exp_time {
            if et < entry_ts {
                panic!(
                    "Expiration time {} < entry time {} for signal bar {}",
                    et, entry_ts, i
                );
            }
        }

        // helper closure to push a new position
        let mut push_pos = |side: &str, tp: f64, sl: f64, size: f64| {
            let entry_price    = if side=="long" {
                price * (1.0 + slippage_rate)
            } else {
                price * (1.0 - slippage_rate)
            };
            let slippage_entry = (entry_price - price).abs();
            let fee_entry      = size * entry_price * entry_fee_rate;

            positions.push(Position {
                position_id:      entry_ts,
                position_type:    side.into(),
                entry_index:      entry_idx,
                entry_price,
                tp,
                sl,
                expiration_time:  exp_time,
                exit_index:       None,
                exit_price:       None,
                exit_condition:   None,
                position_size:    size,
                fee_entry,
                fee_exit:         0.0,
                slippage_entry,
                slippage_exit:    0.0,
                absolute_return:  None,
                real_return:      None,
                pnl:              None,
                is_closed:        false,
            });
        };

        if long[i] {
            push_pos("long", long_tp[i], long_sl[i], long_size[i]);
        } else {
            push_pos("short", short_tp[i], short_sl[i], short_size[i]);
        }
    }

    positions
}
