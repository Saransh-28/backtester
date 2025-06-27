from backtester import run_backtest
import numpy as np

N = 11

# 1) Prepare your bar series (timestamps in float seconds):
ts          = np.arange(N, dtype=float)
open_, high = np.random.rand(N), np.random.rand(N)
low, close  = np.random.rand(N), np.random.rand(N)

# 2) Signals & Levels:
long_sig    = np.zeros(N, dtype=bool)
short_sig   = np.zeros(N, dtype=bool)
long_sig[10]  = True
long_tp     = close * 1.01
long_sl     = close * 0.99
short_tp    = close * 0.99
short_sl    = close * 1.01

# 3) Sizes, Fees, Slippage, Expiration:
long_size       = np.ones(N) * 100
short_size      = np.ones(N) * 100
entry_fee_rate  = 0.0005
exit_fee_rate   = 0.0005
slippage_rate   = 0.0002
expiration_times= ts + 3600.0  # 1-hour TTL
initial_equity  = 10_000.0

# 4) Run:
out = run_backtest(
    timestamp        = ts,
    open             = open_,
    high             = high,
    low              = low,
    close            = close,
    long_signals     = long_sig,
    short_signals    = short_sig,
    long_tp          = long_tp,
    long_sl          = long_sl,
    short_tp         = short_tp,
    short_sl         = short_sl,
    long_size        = long_size,
    short_size       = short_size,
    expiration_times = expiration_times,
    entry_fee_rate   = entry_fee_rate,
    exit_fee_rate    = exit_fee_rate,
    slippage_rate    = slippage_rate,
    initial_equity   = initial_equity,
)

# 5) Inspect results
closed = out["closed_positions"]      # list of realized trades
open_  = out["open_positions"]        # list of still-open trades
expo   = out["exposure_time_series"]  # bar-by-bar exposures & PnL
metrics= out["metrics"]               # overall, long, short metrics

print(closed)
print(open_)
print(expo)
print(metrics)
