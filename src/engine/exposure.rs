// src/engine/exposure.rs

use crate::engine::position::Position;

/// A snapshot of exposure and PnL at one timestamp.
pub struct ExposureSnapshot {
    pub timestamp:       f64,
    pub long_exposure:   f64,  // units
    pub short_exposure:  f64,  // units
    pub total_exposure:  f64,  // units
    pub realized_equity: f64,  // $ realized PnL
    pub floating_pnl:    f64,  // $ unrealized PnL
    pub total_equity:    f64,  // realized + floating
}

pub fn compute_exposure_series(
    positions: &[Position],
    price: &[f64],
    timestamps: &[f64],
    _initial_equity: f64,  // unused in pure-PnL mode
) -> Vec<ExposureSnapshot> {
    let n = price.len();
    // 1) Build event arrays in O(M)
    let mut realized_events = vec![0.0; n];
    let mut long_delta     = vec![0.0; n];
    let mut short_delta    = vec![0.0; n];

    for pos in positions {
        // record realized PnL at exit
        if let Some(exit_i) = pos.exit_index {
            realized_events[exit_i] += pos.pnl.unwrap_or(0.0);
            if pos.position_type == "long" {
                long_delta[exit_i] -= pos.position_size;
            } else {
                short_delta[exit_i] -= pos.position_size;
            }
        }
        // add exposure at entry
        if pos.position_type == "long" {
            long_delta[pos.entry_index] += pos.position_size;
        } else {
            short_delta[pos.entry_index] += pos.position_size;
        }
    }

    // 2) Prefix sums + small per-bar loop
    let mut snapshots    = Vec::with_capacity(n);
    let mut cum_realized = 0.0;
    let mut long_exp     = 0.0;
    let mut short_exp    = 0.0;

    for i in 0..n {
        cum_realized += realized_events[i];
        long_exp     += long_delta[i];
        short_exp    += short_delta[i];

        // compute floating PnL only for still-open positions
        let mut float_pnl = 0.0;
        for pos in positions.iter().filter(|p| {
            p.entry_index <= i && p.exit_index.map_or(true, |ei| ei > i)
        }) {
            if pos.position_type == "long" {
                float_pnl += (price[i] - pos.entry_price) * pos.position_size;
            } else {
                float_pnl += (pos.entry_price - price[i]) * pos.position_size;
            }
        }

        snapshots.push(ExposureSnapshot {
            timestamp:       timestamps[i],
            long_exposure:   long_exp,
            short_exposure:  short_exp,
            total_exposure:  long_exp + short_exp,
            realized_equity: cum_realized,
            floating_pnl:    float_pnl,                  // <-- bind correctly here
            total_equity:    cum_realized + float_pnl,
        });
    }

    snapshots
}
