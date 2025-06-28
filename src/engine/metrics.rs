use crate::engine::position::Position;
use crate::engine::exposure::ExposureSnapshot;

/// Per‐trade metrics (notional‐normalized returns)
#[derive(Debug)]
pub struct SideTradeMetrics {
    pub number_of_trades:     usize,
    pub win_rate:             f64,
    pub loss_rate:            f64,
    pub average_trade_return: f64,
    pub average_trade_pnl:    f64,
    pub profit_factor:        f64,
    pub expectancy:           f64,
    pub average_duration:     f64,
    pub trade_returns:        Vec<f64>,
    pub trade_pnls:           Vec<f64>,
    pub durations:            Vec<f64>,
}

/// Bar‐by‐bar portfolio metrics
#[derive(Debug, Clone)]
pub struct TimeSeriesMetrics {
    pub returns:           Vec<f64>, // R_t per bar
    pub mean_return:       f64,
    pub volatility:        f64,
    pub sharpe_ratio:      f64,
    pub cumulative_return: f64,
    pub max_drawdown:      f64,
}

/// Combined side metrics
#[derive(Debug)]
pub struct SideMetrics {
    pub total_return:  f64,
    pub total_pnl:     f64,
    pub trade_metrics: SideTradeMetrics,
    pub time_metrics:  TimeSeriesMetrics,
}

/// All‐sides container
#[derive(Debug)]
pub struct SummaryMetrics {
    pub overall: SideMetrics,
    pub longs:   SideMetrics,
    pub shorts:  SideMetrics,
}

/// Build just the trade‐level slice
fn compute_trade_metrics(
    trades: Vec<&Position>,
) -> SideTradeMetrics {
    let mut ordered = trades;
    ordered.sort_by_key(|p| p.exit_index.unwrap_or(usize::MAX));

    let n = ordered.len();
    let mut trade_returns = Vec::with_capacity(n);
    let mut trade_pnls    = Vec::with_capacity(n);
    let mut durations     = Vec::with_capacity(n);

    let mut sum_wins   = 0.0_f64;
    let mut sum_losses = 0.0_f64;
    let mut wins       = 0;
    let mut losses     = 0;

    for &pos in &ordered {
        let pnl = pos.pnl.unwrap_or(0.0);
        trade_pnls.push(pnl);

        // r_i = PnL_i / (entry_price * position_size)
        let notional = pos.entry_price * pos.position_size;
        let r = if notional != 0.0 {
            pnl / notional
        } else {
            0.0
        };
        trade_returns.push(r);

        if pnl > 0.0 {
            sum_wins += pnl;
            wins += 1;
        } else if pnl < 0.0 {
            sum_losses += -pnl;
            losses += 1;
        }

        // duration in bars
        let dur = (pos.exit_index.unwrap() as isize - pos.entry_index as isize).abs() as f64;
        durations.push(dur);
    }

    let nf = n as f64;
    let win_rate  = if nf > 0.0 { wins as f64 / nf } else { 0.0 };
    let loss_rate = if nf > 0.0 { losses as f64 / nf } else { 0.0 };
    let avg_ret   = if nf > 0.0 { trade_returns.iter().sum::<f64>() / nf } else { 0.0 };
    let avg_pnl   = if nf > 0.0 { trade_pnls.iter().sum::<f64>() / nf } else { 0.0 };
    let profit_factor = 
        if sum_losses > 0.0 { sum_wins / sum_losses } else { f64::INFINITY };
    let expectancy    = avg_ret;
    let avg_dur       = if nf > 0.0 { durations.iter().sum::<f64>() / nf } else { 0.0 };

    SideTradeMetrics {
        number_of_trades:     n,
        win_rate,
        loss_rate,
        average_trade_return: avg_ret,
        average_trade_pnl:    avg_pnl,
        profit_factor,
        expectancy,
        average_duration:     avg_dur,
        trade_returns,
        trade_pnls,
        durations,
    }
}

/// Build bar‐by‐bar metrics from the **full** exposure curve
fn compute_time_metrics(
    exposure: &[ExposureSnapshot],
) -> TimeSeriesMetrics {
    let n = exposure.len();
    let mut returns = Vec::with_capacity(n.saturating_sub(1));

    for i in 1..n {
        let prev = exposure[i - 1].total_equity;
        let cur  = exposure[i].total_equity;
        let r    = if prev != 0.0 {
            (cur - prev) / prev
        } else {
            0.0
        };
        returns.push(r);
    }

    let m = returns.len() as f64;
    let mean_return = if m > 0.0 { returns.iter().sum::<f64>() / m } else { 0.0 };
    let volatility  = if m > 1.0 {
        let mu = mean_return;
        (returns.iter().map(|&x| (x - mu).powi(2)).sum::<f64>() / (m - 1.0)).sqrt()
    } else {
        0.0
    };
    let sharpe_ratio = if volatility != 0.0 { mean_return / volatility } else { 0.0 };

    // cumulative = (E_final / E_initial) - 1
    let cum_return = if exposure[0].total_equity != 0.0 {
        (exposure[n - 1].total_equity / exposure[0].total_equity) - 1.0
    } else {
        0.0
    };

    // max drawdown
    let mut peak: f64   = exposure[0].total_equity;
    let mut max_dd: f64 = 0.0;
    for snap in exposure {
        let eq = snap.total_equity;
        peak = peak.max(eq);
        let dd = if peak != 0.0 { (peak - eq) / peak } else { 0.0 };
        max_dd = max_dd.max(dd);
    }

    TimeSeriesMetrics {
        returns,
        mean_return,
        volatility,
        sharpe_ratio,
        cumulative_return: cum_return,
        max_drawdown:      max_dd,
    }
}

/// Top‐level: per‐trade + time‐series for overall, longs, shorts
pub fn compute_summary_metrics(
    _initial_equity: f64,
    closed: &[Position],
    exposure: &[ExposureSnapshot],
) -> SummaryMetrics {
    // partition the closed trades
    let all:   Vec<&Position> = closed.iter().collect();
    let longs: Vec<&Position> = closed.iter().filter(|p| p.position_type == "long").collect();
    let shorts:Vec<&Position> = closed.iter().filter(|p| p.position_type == "short").collect();

    // trade metrics
    let tm_all   = compute_trade_metrics(all.clone());
    let tm_long  = compute_trade_metrics(longs.clone());
    let tm_short = compute_trade_metrics(shorts.clone());

    // time metrics (one full exposure curve)
    let ts_all = compute_time_metrics(exposure);

    // total PnL from exposure
    let final_snap = exposure.last().unwrap();
    let total_pnl  = final_snap.realized_equity + final_snap.floating_pnl;
    let total_ret  = ts_all.cumulative_return;

    SummaryMetrics {
        overall: SideMetrics {
            total_return:  total_ret,
            total_pnl,
            trade_metrics: tm_all,
            time_metrics:  ts_all.clone(),
        },
        longs: SideMetrics {
            total_return:  total_ret,
            total_pnl:     tm_long.trade_pnls.iter().sum(),
            trade_metrics: tm_long,
            time_metrics:  ts_all.clone(),
        },
        shorts: SideMetrics {
            total_return:  total_ret,
            total_pnl:     tm_short.trade_pnls.iter().sum(),
            trade_metrics: tm_short,
            time_metrics:  ts_all.clone(),
        },
    }
}
