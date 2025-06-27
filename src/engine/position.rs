// src/engine/position.rs

#[derive(Clone, Debug)]
pub struct Position {
    /// The entry timestamp (UNIX seconds) of this position
    pub position_id:        f64,
    /// "long" or "short"
    pub position_type:      String,
    /// Bar‐index at which this position was filled
    pub entry_index:        usize,
    /// Fill price (includes slippage)
    pub entry_price:        f64,
    /// Absolute take‐profit level
    pub tp:                 f64,
    /// Absolute stop‐loss level
    pub sl:                 f64,
    /// Optional expiration timestamp (must be ≥ position_id)
    pub expiration_time:    Option<f64>,
    /// Bar‐index at which this position was closed
    pub exit_index:         Option<usize>,
    /// Fill price at exit (includes slippage)
    pub exit_price:         Option<f64>,
    /// "TP", "SL", or "EXP"
    pub exit_condition:     Option<String>,
    /// Number of units/contracts
    pub position_size:      f64,
    /// $ fee charged at entry
    pub fee_entry:          f64,
    /// $ fee charged at exit
    pub fee_exit:           f64,
    /// Price‐delta slippage at entry (reporting only)
    pub slippage_entry:     f64,
    /// Price‐delta slippage at exit (reporting only)
    pub slippage_exit:      f64,
    /// (exit_price/entry_price − 1)
    pub absolute_return:    Option<f64>,
    /// net $ PnL / (entry_price×units)
    pub real_return:        Option<f64>,
    /// net $ PnL
    pub pnl:                Option<f64>,
    /// true once closed
    pub is_closed:          bool,
}
