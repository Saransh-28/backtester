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
    metrics::{compute_summary_metrics, SideTradeMetrics, TimeSeriesMetrics},
    position::Position,
};

/// Ensure `arr.len() == expected`, otherwise PyValueError
fn validate_length<T>(arr: &Vec<T>, name: &str, expected: usize) -> PyResult<()> {
    if arr.len() != expected {
        Err(PyValueError::new_err(format!(
            "‘{}’ length {} != expected {}",
            name, arr.len(), expected
        )))
    } else {
        Ok(())
    }
}

#[pyfunction]
#[pyo3(signature=(
    timestamp, open, high, low, close,
    long_signals, short_signals,
    long_tp, long_sl, short_tp, short_sl,
    long_size, short_size,
    expiration_times,
    entry_fee_rate, exit_fee_rate, slippage_rate,
    initial_equity
))]
pub fn run_backtest(
    py: Python<'_>,
    timestamp:        &PyArray1<f64>,
    open:             &PyArray1<f64>,
    high:             &PyArray1<f64>,
    low:              &PyArray1<f64>,
    close:            &PyArray1<f64>,
    long_signals:     &PyArray1<bool>,
    short_signals:    &PyArray1<bool>,
    long_tp:          &PyArray1<f64>,
    long_sl:          &PyArray1<f64>,
    short_tp:         &PyArray1<f64>,
    short_sl:         &PyArray1<f64>,
    long_size:        &PyArray1<f64>,
    short_size:       &PyArray1<f64>,
    expiration_times: &PyArray1<f64>,
    entry_fee_rate:   f64,
    exit_fee_rate:    f64,
    slippage_rate:    f64,
    initial_equity:   f64,
) -> PyResult<PyObject> {
    // 1) Pull into Rust Vecs
    let mut ts        = unsafe { timestamp.as_slice()? }.to_vec();
    if !ts.windows(2).all(|w| w[1] > w[0]) {
        return Err(PyValueError::new_err("timestamps must be strictly increasing"));
    }
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

    // 1b) Signal mutual‐exclusion
    for i in 0..ts.len() {
        if long_sig[i] && short_sig[i] {
            return Err(PyValueError::new_err(format!(
                "both long and short signals true at index {}", i
            )));
        }
    }

    // 2) Validate core lengths
    let n = prepare_inputs(&mut [&mut ts, &mut o, &mut h, &mut l, &mut c])
        .map_err(PyValueError::new_err)?;
    validate_length(&long_sig,  "long_signals",     n)?;
    validate_length(&short_sig, "short_signals",    n)?;
    validate_length(&l_tp_vec,  "long_tp",          n)?;
    validate_length(&l_sl_vec,  "long_sl",          n)?;
    validate_length(&s_tp_vec,  "short_tp",         n)?;
    validate_length(&s_sl_vec,  "short_sl",         n)?;
    validate_length(&l_sz,      "long_size",        n)?;
    validate_length(&s_sz,      "short_size",       n)?;
    validate_length(&exp_times, "expiration_times", n)?;

    // 2b) Expirations must not precede their bar‐timestamp
    for i in 0..n {
        if exp_times[i] < ts[i] {
            return Err(PyValueError::new_err(format!(
                "expiration_time {} < timestamp {} at index {}",
                exp_times[i], ts[i], i
            )));
        }
    }

    // 3) Entries
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

    // 4) Exits
    simulate_position_exits(&mut positions, &ts, &h, &l, &c, exit_fee_rate, slippage_rate);

    // 5) Exposure & metrics
    let exposure_series = compute_exposure_series(&positions, &c, &ts, initial_equity);
    let closed: Vec<Position> = positions.iter().cloned().filter(|p| p.is_closed).collect();
    let open_: Vec<Position>   = positions.iter().cloned().filter(|p| !p.is_closed).collect();
    let summary_metrics = compute_summary_metrics(initial_equity, &closed, &exposure_series);

    // 6) Marshal Python output
    let out = PyDict::new(py);

    // 6a) closed_positions
    let py_closed = PyList::empty(py);
    for pos in &closed {
        let pd = PyDict::new(py);
        pd.set_item("position_id",     pos.position_id)?;
        pd.set_item("position_type",   &pos.position_type)?;
        pd.set_item("entry_index",     pos.entry_index)?;
        pd.set_item("entry_price",     pos.entry_price)?;
        pd.set_item("tp",              pos.tp)?;
        pd.set_item("sl",              pos.sl)?;
        pd.set_item("expiration_time", pos.expiration_time)?;
        pd.set_item("exit_index",      pos.exit_index)?;
        pd.set_item("exit_price",      pos.exit_price)?;
        pd.set_item("exit_condition",  &pos.exit_condition)?;
        pd.set_item("position_size",   pos.position_size)?;
        pd.set_item("fee_entry",       pos.fee_entry)?;
        pd.set_item("slippage_entry",  pos.slippage_entry)?;
        pd.set_item("fee_exit",        pos.fee_exit)?;
        pd.set_item("slippage_exit",   pos.slippage_exit)?;
        pd.set_item("absolute_return", pos.absolute_return)?;
        pd.set_item("real_return",     pos.real_return)?;
        pd.set_item("pnl",             pos.pnl)?;
        pd.set_item("is_closed",       pos.is_closed)?;
        py_closed.append(pd)?;
    }
    out.set_item("closed_positions", py_closed)?;

    // 6b) open_positions
    let py_open = PyList::empty(py);
    for pos in &open_ {
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

    // 6c) exposure_time_series
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

    // 6d) metrics
    let to_py_trade = |py: Python<'_>, tm: &SideTradeMetrics| -> PyResult<PyObject> {
        let d = PyDict::new(py);
        d.set_item("number_of_trades",     tm.number_of_trades)?;
        d.set_item("win_rate",             tm.win_rate)?;
        d.set_item("loss_rate",            tm.loss_rate)?;
        d.set_item("average_trade_return", tm.average_trade_return)?;
        d.set_item("average_trade_pnl",    tm.average_trade_pnl)?;
        d.set_item("profit_factor",        tm.profit_factor)?;
        d.set_item("expectancy",           tm.expectancy)?;
        d.set_item("average_duration",     tm.average_duration)?;
        d.set_item("trade_returns", PyList::new(py, &tm.trade_returns))?;
        d.set_item("trade_pnls",    PyList::new(py, &tm.trade_pnls))?;
        d.set_item("durations",     PyList::new(py, &tm.durations))?;
        Ok(d.into())
    };
    let to_py_time = |py: Python<'_>, tsm: &TimeSeriesMetrics| -> PyResult<PyObject> {
        let d = PyDict::new(py);
        d.set_item("returns",           PyList::new(py, &tsm.returns))?;
        d.set_item("mean_return",       tsm.mean_return)?;
        d.set_item("volatility",        tsm.volatility)?;
        d.set_item("sharpe_ratio",      tsm.sharpe_ratio)?;
        d.set_item("cumulative_return", tsm.cumulative_return)?;
        d.set_item("max_drawdown",      tsm.max_drawdown)?;
        Ok(d.into())
    };

    let pm = PyDict::new(py);

    let om = &summary_metrics.overall;
    let d_om = PyDict::new(py);
    d_om.set_item("total_return",  om.total_return)?;
    d_om.set_item("total_pnl",     om.total_pnl)?;
    d_om.set_item("trade_metrics", to_py_trade(py, &om.trade_metrics)?)?;
    d_om.set_item("time_metrics",  to_py_time(py, &om.time_metrics)?)?;
    pm.set_item("overall", d_om)?;

    let lm = &summary_metrics.longs;
    let d_lm = PyDict::new(py);
    d_lm.set_item("total_return",  lm.total_return)?;
    d_lm.set_item("total_pnl",     lm.total_pnl)?;
    d_lm.set_item("trade_metrics", to_py_trade(py, &lm.trade_metrics)?)?;
    d_lm.set_item("time_metrics",  to_py_time(py, &lm.time_metrics)?)?;
    pm.set_item("long", d_lm)?;

    let sm = &summary_metrics.shorts;
    let d_sm = PyDict::new(py);
    d_sm.set_item("total_return",  sm.total_return)?;
    d_sm.set_item("total_pnl",     sm.total_pnl)?;
    d_sm.set_item("trade_metrics", to_py_trade(py, &sm.trade_metrics)?)?;
    d_sm.set_item("time_metrics",  to_py_time(py, &sm.time_metrics)?)?;
    pm.set_item("short", d_sm)?;

    out.set_item("metrics", pm)?;
    Ok(out.into())
}
