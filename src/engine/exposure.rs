// src/engine/exposure.rs

use crate::engine::position::Position;

pub struct ExposureSnapshot {
    pub timestamp:       f64,
    pub long_exposure:   f64,  // sum of open long units
    pub short_exposure:  f64,  // sum of open short units
    pub total_exposure:  f64,  // long_exposure + short_exposure
    pub realized_equity: f64,  // cumulated realized PnL (size×return)
    pub floating_pnl:    f64,  // instantaneous unrealized PnL
    pub total_equity:    f64,  // initial_equity + realized_equity + floating_pnl
}

pub fn compute_exposure_series(
    positions: &[Position],
    price: &[f64],
    timestamps: &[f64],
    initial_equity: f64,
) -> Vec<ExposureSnapshot> {
    let mut snapshots    = Vec::with_capacity(timestamps.len());
    let mut realized_pnl = 0.0;  // we'll accumulate each closed trade's pnl here

    for (i, &t) in timestamps.iter().enumerate() {
        // 1) First, realize PnL of any trade that exits on this bar
        for pos in positions {
            if let Some(exit_i) = pos.exit_index {
                if exit_i == i {
                    // pos.pnl is in $, so this is cumulated realized PnL
                    realized_pnl += pos.pnl.unwrap_or(0.0);
                }
            }
        }

        // 2) Now, for each bar, accumulate unit‐based exposure & floating PnL
        let mut long_units  = 0.0;
        let mut short_units = 0.0;
        let mut float_pnl   = 0.0;

        for pos in positions {
            let entered   = pos.entry_index <= i;
            let still_live = pos.exit_index.map_or(true, |exit_i| exit_i > i);
            if entered && still_live {
                // unit exposure
                let units = pos.position_size;
                if pos.position_type == "long" {
                    long_units  += units;
                    // unrealized PnL = (mark - entry_fill) * units
                    float_pnl   += (price[i] - pos.entry_price) * units;
                } else {
                    short_units += units;
                    float_pnl   += (pos.entry_price - price[i]) * units;
                }
            }
        }

        // 3) Push the snapshot
        snapshots.push(ExposureSnapshot {
            timestamp:       t,
            long_exposure:   long_units,
            short_exposure:  short_units,
            total_exposure:  long_units + short_units,
            realized_equity: realized_pnl,
            floating_pnl:    float_pnl,
            total_equity:    initial_equity + realized_pnl + float_pnl,
        });
    }

    snapshots
}
