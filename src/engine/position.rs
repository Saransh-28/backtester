// src/engine/position.rs

#[derive(Clone, Debug)]
pub struct Position {
    pub position_id:        usize,          // entry bar index
    pub position_type:      String,         // "long" or "short"
    pub entry_index:        usize,          // bar of fill
    pub entry_price:        f64,            // fill price (includes slippage)
    pub tp:                 f64,            // raw TP price level
    pub sl:                 f64,            // raw SL price level
    pub expiration_time:    Option<f64>,    // absolute timestamp to force EXP
    pub exit_index:         Option<usize>,  // bar of fill
    pub exit_price:         Option<f64>,    // fill price (includes slippage)
    pub exit_condition:     Option<String>, // "TP", "SL", or "EXP"
    pub position_size:      f64,            // number of units/contracts
    pub fee_entry:          f64,            // $ cost at entry
    pub fee_exit:           f64,            // $ cost at exit
    pub slippage_entry:     f64,            // price delta at entry
    pub slippage_exit:      f64,            // price delta at exit
    pub absolute_return:    Option<f64>,    // (exit_price/entry_price - 1)
    pub real_return:        Option<f64>,    // net $ PnL / (entry_price*units)
    pub pnl:                Option<f64>,    // net $ PnL
    pub is_closed:          bool,
}
