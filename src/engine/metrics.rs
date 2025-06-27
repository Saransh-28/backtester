// metrics.rs

use crate::engine::position::Position;

/// Metrics for one “side” of the strategy (or the overall):
#[derive(Debug)]
pub struct SideMetrics {
    pub total_return:     f64,
    pub total_pnl:        f64,
    pub sharpe_ratio:     f64,
    pub max_drawdown:     f64,
    pub win_rate:         f64,
    pub number_of_trades: usize,
    pub average_return:   f64,
    pub average_pnl:      f64,
}

/// Aggregated metrics for long, short, and overall:
#[derive(Debug)]
pub struct SummaryMetrics {
    pub overall: SideMetrics,
    pub longs:   SideMetrics,
    pub shorts:  SideMetrics,
}

/// Compute equity‐curve–based metrics for a slice of closed positions:
fn compute_side_metrics(
    initial_equity: f64,
    mut trades: Vec<&Position>,
) -> SideMetrics {
    // Sort by exit time:
    trades.sort_by_key(|p| p.exit_index.unwrap());

    // Build equity curve in currency units:
    let mut eq_curve = Vec::with_capacity(trades.len() + 1);
    eq_curve.push(initial_equity);
    for &pos in &trades {
        let last = *eq_curve.last().unwrap();
        let pnl  = pos.pnl.unwrap_or(0.0);
        eq_curve.push(last + pnl);
    }

    // Per‐trade returns on evolving equity:
    let mut rets = Vec::with_capacity(trades.len());
    for i in 1..eq_curve.len() {
        let prev = eq_curve[i - 1];
        let cur  = eq_curve[i];
        let r    = if prev > 0.0 { (cur - prev) / prev } else { 0.0 };
        rets.push(r);
    }

    let n = rets.len() as f64;
    let final_eq     = *eq_curve.last().unwrap();
    let total_pnl    = final_eq - initial_equity;
    let total_return = if initial_equity > 0.0 {
        (final_eq / initial_equity) - 1.0
    } else { 0.0 };

    // Sharpe = mean / std
    let mean = if n > 0.0 { rets.iter().sum::<f64>() / n } else { 0.0 };
    let std  = if n > 1.0 {
        (rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else { 0.0 };
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };

    // Win rate
    let wins     = rets.iter().filter(|&&r| r > 0.0).count() as f64;
    let win_rate = if n > 0.0 { wins / n } else { 0.0 };

    // Max drawdown on equity curve
    let mut peak   = eq_curve[0];
    let mut max_dd = 0.0;
    for &eq in &eq_curve {
        if eq > peak { peak = eq; }
        let dd = if peak > 0.0 { (peak - eq) / peak } else { 0.0 };
        if dd > max_dd { max_dd = dd; }
    }

    SideMetrics {
        total_return,
        total_pnl,
        sharpe_ratio:     sharpe,
        max_drawdown:     max_dd,
        win_rate,
        number_of_trades: rets.len(),
        average_return:   mean,
        average_pnl:      if rets.len() > 0 { total_pnl / rets.len() as f64 } else { 0.0 },
    }
}

/// The new top‐level entry point: 
/// compute overall, long‐only, and short‐only metrics in one go.
pub fn compute_summary_metrics(
    initial_equity: f64,
    closed: &[Position],
) -> SummaryMetrics {
    // Collect references so we don’t clone Positions more than needed:
    let all_trades   = closed.iter().collect::<Vec<_>>();
    let long_trades  = closed.iter().filter(|p| p.position_type == "long").collect();
    let short_trades = closed.iter().filter(|p| p.position_type == "short").collect();

    SummaryMetrics {
        overall: compute_side_metrics(initial_equity, all_trades),
        longs:   compute_side_metrics(initial_equity, long_trades),
        shorts:  compute_side_metrics(initial_equity, short_trades),
    }
}
