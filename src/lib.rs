// src/lib.rs

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

mod engine;

#[pymodule]
fn backtester(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(engine::run_backtest, m)?)?;
    Ok(())
}
