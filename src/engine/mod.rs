// src/engine/mod.rs

pub mod position;
pub mod prepare_inputs;
pub mod scan_entries;
pub mod simulate_exits;
pub mod exposure;
pub mod metrics;

use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::exceptions::PyValueError;

use crate::engine::{
    prepare_inputs::prepare_inputs,
    scan_entries::scan_entries,
    simulate_exits::simulate_position_exits,
    exposure::compute_exposure_series,
    metrics::{compute_summary_metrics, SideMetrics},
    position::Position,
};

#[pyfunction]
#[pyo3(signature=(
    timestamp, open, high, low, close,
    long_signals, short_signals,
    long_tp, long_sl, short_tp, short_sl,
    long_size, short_size,
    expiration_times,         // ← now f64 timestamps
    entry_fee_rate, exit_fee_rate, slippage_rate,
    initial_equity
))]
pub fn run_backtest(
    py: Python<'_>,
    timestamp:         &PyArray1<f64>,
    open:              &PyArray1<f64>,
    high:              &PyArray1<f64>,
    low:               &PyArray1<f64>,
    close:             &PyArray1<f64>,
    long_signals:      &PyArray1<bool>,
    short_signals:     &PyArray1<bool>,
    long_tp:           &PyArray1<f64>,
    long_sl:           &PyArray1<f64>,
    short_tp:          &PyArray1<f64>,
    short_sl:          &PyArray1<f64>,
    long_size:         &PyArray1<f64>,
    short_size:        &PyArray1<f64>,
    expiration_times:  &PyArray1<f64>, // ← change here
    entry_fee_rate:    f64,
    exit_fee_rate:     f64,
    slippage_rate:     f64,
    initial_equity:    f64,
) -> PyResult<PyObject> {
    // 1) Into Vecs
    let mut ts        = unsafe { timestamp.as_slice()? }.to_vec();
    let mut o         = unsafe { open.as_slice()? }.to_vec();
    let mut h         = unsafe { high.as_slice()? }.to_vec();
    let mut l         = unsafe { low.as_slice()? }.to_vec();
    let mut c         = unsafe { close.as_slice()? }.to_vec();
    let long_sig      = unsafe { long_signals.as_slice()? }.to_vec();
    let short_sig     = unsafe { short_signals.as_slice()? }.to_vec();
    let l_tp_vec      = unsafe { long_tp.as_slice()? }.to_vec();
    let l_sl_vec      = unsafe { long_sl.as_slice()? }.to_vec();
    let s_tp_vec      = unsafe { short_tp.as_slice()? }.to_vec();
    let s_sl_vec      = unsafe { short_sl.as_slice()? }.to_vec();
    let l_sz          = unsafe { long_size.as_slice()? }.to_vec();
    let s_sz          = unsafe { short_size.as_slice()? }.to_vec();
    let exp_times     = unsafe { expiration_times.as_slice()? }.to_vec();

    // 2) Validate lengths
    let len = prepare_inputs(&mut [&mut ts, &mut o, &mut h, &mut l, &mut c])
        .map_err(PyValueError::new_err)?;
    if long_sig.len()!=len || short_sig.len()!=len
       || l_tp_vec.len()!=len || l_sl_vec.len()!=len
       || s_tp_vec.len()!=len || s_sl_vec.len()!=len
       || l_sz.len()!=len     || s_sz.len()!=len
       || exp_times.len()!=len {
        return Err(PyValueError::new_err("All input arrays must have the same length"));
    }

    // 3) Scan entries with real‐time expiry
    let mut positions = scan_entries(
        &ts,
        &o, &long_sig, &short_sig,
        &l_tp_vec, &l_sl_vec,
        &s_tp_vec, &s_sl_vec,
        &l_sz, &s_sz,
        &exp_times,
        entry_fee_rate,
        slippage_rate,
    );

    // 4) Simulate exits (now needs timestamps too)
    simulate_position_exits(&mut positions, &ts, &h, &l, &c, exit_fee_rate, slippage_rate);

    // 5) Exposure & metrics
    let exposure_series = compute_exposure_series(&positions, &c, &ts, initial_equity);
    let closed: Vec<Position> = positions.iter().cloned().filter(|p| p.is_closed).collect();
    let open_:  Vec<Position> = positions.iter().cloned().filter(|p| !p.is_closed).collect();
    let m_all   = compute_summary_metrics(initial_equity, &closed);

    // 6) Build Python dict…
    let out = PyDict::new(py);

    // closed_positions…
    let py_closed = PyList::empty(py);
    for pos in &closed {
        let pd = PyDict::new(py);
        pd.set_item("position_id",      pos.position_id)?;
        pd.set_item("position_type",    &pos.position_type)?;
        pd.set_item("entry_index",      pos.entry_index)?;
        pd.set_item("entry_price",      pos.entry_price)?;
        pd.set_item("tp",               pos.tp)?;
        pd.set_item("sl",               pos.sl)?;
        pd.set_item("expiration_time", pos.expiration_time)?;
        pd.set_item("exit_index",       pos.exit_index)?;
        pd.set_item("exit_price",       pos.exit_price)?;
        pd.set_item("exit_condition",   &pos.exit_condition)?;
        pd.set_item("position_size",    pos.position_size)?;
        pd.set_item("fee_entry",        pos.fee_entry)?;
        pd.set_item("slippage_entry",   pos.slippage_entry)?;
        pd.set_item("fee_exit",         pos.fee_exit)?;
        pd.set_item("slippage_exit",    pos.slippage_exit)?;
        pd.set_item("absolute_return",  pos.absolute_return)?;
        pd.set_item("real_return",      pos.real_return)?;
        pd.set_item("pnl",              pos.pnl)?;
        pd.set_item("is_closed",        pos.is_closed)?;
        py_closed.append(pd)?;
    }
    out.set_item("closed_positions", py_closed)?;

    let py_open = PyList::empty(py);
    for pos in open_.iter() {
        let pd = PyDict::new(py);
        pd.set_item("position_id",     pos.position_id)?;
        pd.set_item("position_type",   &pos.position_type)?;
        pd.set_item("entry_index",     pos.entry_index)?;
        pd.set_item("entry_price",     pos.entry_price)?;
        pd.set_item("tp",              pos.tp)?;
        pd.set_item("sl",              pos.sl)?;
        pd.set_item("expiration_time", pos.expiration_time)?;
        pd.set_item("position_size",   pos.position_size)?;
        pd.set_item("fee_entry",       pos.fee_entry)?;
        pd.set_item("slippage_entry",  pos.slippage_entry)?;
        pd.set_item("is_closed",       pos.is_closed)?;
        py_open.append(pd)?;
    }
    out.set_item("open_positions", py_open)?;

    // exposure_time_series…
    let py_expo = PyList::empty(py);
    for snap in &exposure_series {
        let pd = PyDict::new(py);
        pd.set_item("timestamp",       snap.timestamp)?;
        pd.set_item("long_exposure",   snap.long_exposure)?;
        pd.set_item("short_exposure",  snap.short_exposure)?;
        pd.set_item("total_exposure",  snap.total_exposure)?;
        pd.set_item("realized_equity", snap.realized_equity)?;
        pd.set_item("floating_pnl",    snap.floating_pnl)?;
        pd.set_item("total_equity",    snap.total_equity)?;
        py_expo.append(pd)?;
    }
    out.set_item("exposure_time_series", py_expo)?;

    // metrics…
    let pm = PyDict::new(py);
    let to_py = |side: &SideMetrics| {
        let d = PyDict::new(py);
        d.set_item("total_return",     side.total_return).unwrap();
        d.set_item("total_pnl",        side.total_pnl).unwrap();
        d.set_item("sharpe_ratio",     side.sharpe_ratio).unwrap();
        d.set_item("max_drawdown",     side.max_drawdown).unwrap();
        d.set_item("win_rate",         side.win_rate).unwrap();
        d.set_item("number_of_trades", side.number_of_trades).unwrap();
        d.set_item("average_return",   side.average_return).unwrap();
        d.set_item("average_pnl",      side.average_pnl).unwrap();
        d
    };
    pm.set_item("overall", to_py(&m_all.overall))?;
    pm.set_item("long",    to_py(&m_all.longs))?;
    pm.set_item("short",   to_py(&m_all.shorts))?;
    out.set_item("metrics", pm)?;

    Ok(out.into())
}
