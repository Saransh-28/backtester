// src/engine/scan_entries.rs

use crate::engine::position::Position;

/// For each signal on bar i, enter on bar i+1 open (or bar i if last bar).
/// Signals must be mutually exclusive.  Expiration_time[k] ≥ timestamps[k].
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

    // 1) Mutual‐exclusion check
    for i in 0..n {
        if long[i] && short[i] {
            panic!("Both long and short signals true at bar {}", i);
        }
    }

    let mut positions = Vec::new();
    for i in 0..n {
        // Determine fill bar (i+1 or last bar)
        let entry_idx = if i + 1 < n { i + 1 } else { i };
        let entry_ts  = timestamps[entry_idx];
        let raw_open  = open[entry_idx];
        let exp_time  = expiration_times.get(entry_idx).copied();

        // 2) Expiration sanity‐check
        if let Some(et) = exp_time {
            if et < entry_ts {
                panic!(
                    "Expiration time {} before entry time {} at bar {}",
                    et, entry_ts, entry_idx
                );
            }
        }

        // LONG entry
        if long[i] {
            let entry_price    = raw_open * (1.0 + slippage_rate);
            let slippage_entry = entry_price - raw_open;
            let fee_entry      = long_size[i] * entry_price * entry_fee_rate;
            positions.push(Position {
                position_id:      entry_ts,
                position_type:    "long".into(),
                entry_index:      entry_idx,
                entry_price,
                tp:               long_tp[i],
                sl:               long_sl[i],
                expiration_time:  exp_time,
                exit_index:       None,
                exit_price:       None,
                exit_condition:   None,
                position_size:    long_size[i],
                fee_entry,
                fee_exit:         0.0,
                slippage_entry,
                slippage_exit:    0.0,
                absolute_return:  None,
                real_return:      None,
                pnl:              None,
                is_closed:        false,
            });
        }

        // SHORT entry
        if short[i] {
            let entry_price    = raw_open * (1.0 - slippage_rate);
            let slippage_entry = raw_open - entry_price;
            let fee_entry      = short_size[i] * entry_price * entry_fee_rate;
            positions.push(Position {
                position_id:      entry_ts,
                position_type:    "short".into(),
                entry_index:      entry_idx,
                entry_price,
                tp:               short_tp[i],
                sl:               short_sl[i],
                expiration_time:  exp_time,
                exit_index:       None,
                exit_price:       None,
                exit_condition:   None,
                position_size:    short_size[i],
                fee_entry,
                fee_exit:         0.0,
                slippage_entry,
                slippage_exit:    0.0,
                absolute_return:  None,
                real_return:      None,
                pnl:              None,
                is_closed:        false,
            });
        }
    }

    positions
}
