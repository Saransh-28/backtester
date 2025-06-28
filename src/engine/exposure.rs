// src/engine/exposure.rs

use crate::engine::position::Position;

/// One snapshot of bar-level exposure + PnL
pub struct ExposureSnapshot {
    pub timestamp:       f64,
    pub long_exposure:   f64,
    pub short_exposure:  f64,
    pub total_exposure:  f64,
    pub realized_equity: f64,
    pub floating_pnl:    f64,
    pub total_equity:    f64,
}

/// O(N + M) exposure/PnL via prefix-sums + small per-bar loops
pub fn compute_exposure_series(
    positions: &[Position],
    price: &[f64],
    timestamps: &[f64],
    _initial_equity: f64,
) -> Vec<ExposureSnapshot> {
    let n = price.len();

    // 1) Build event arrays
    let mut realized_events = vec![0.0; n];
    let mut long_delta      = vec![0.0; n];
    let mut short_delta     = vec![0.0; n];

    for pos in positions {
        // When the trade exits, realize its PnL
        if let Some(exit_i) = pos.exit_index {
            realized_events[exit_i] += pos.pnl.unwrap_or(0.0);
            if pos.position_type=="long" {
                long_delta[exit_i] -= pos.position_size;
            } else {
                short_delta[exit_i] -= pos.position_size;
            }
        }
        // At entry, add exposure
        if pos.position_type=="long" {
            long_delta[pos.entry_index] += pos.position_size;
        } else {
            short_delta[pos.entry_index] += pos.position_size;
        }
    }

    // 2) Prefix‐sum + per‐bar floating PnL
    let mut snapshots    = Vec::with_capacity(n);
    let mut cum_realized = 0.0;
    let mut long_exp     = 0.0;
    let mut short_exp    = 0.0;

    for i in 0..n {
        cum_realized += realized_events[i];
        long_exp     += long_delta[i];
        short_exp    += short_delta[i];

        // Only **open** positions contribute to floating
        let mut float_pnl = 0.0;
        for pos in positions.iter().filter(|p| {
            p.entry_index <= i && p.exit_index.map_or(true, |ei| ei > i)
        }) {
            if pos.position_type=="long" {
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
            floating_pnl:    float_pnl,
            total_equity:    cum_realized + float_pnl,
        });
    }

    snapshots
}
