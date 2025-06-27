# Backtester

A **modular, rule-based backtesting engine** written in Rust with Python bindings via PyO3 and Maturin. Designed for clarity, performance, and flexibility, it treats **each signal entry** as an **independent position** and simulates PnL, exposure, and performance metrics on a bar-by-bar basis.

---

## 🚀 Features & Assumptions

- **Independent positions**  
  Every `long` or `short` signal spawns a new, standalone position—no netting or aggregation.

- **Absolute TP/SL/Expiration**  
  - **Take-profit** and **stop-loss** are absolute price levels.  
  - **Expiration** is an optional timestamp after which the position is force-closed if neither TP nor SL has hit.

- **Per-position fees & slippage**  
  - **Entry/exit fees** are applied on the notional traded (`units × fill_price × fee_rate`).  
  - **Slippage** is modeled as a price‐delta on fill (e.g. `fill_price = raw_price × (1 ± slippage_rate)`), then used in PnL.

- **Bar-by-bar Equity & Exposure**  
  - **Exposure** = sum of open units on each side at each bar.  
  - **Floating PnL** = unrealized PnL at mark price.  
  - **Realized PnL** = cumulative dollar PnL of all closed trades.  
  - **Total PnL curve** = initial equity + realized + floating.

- **Performance Metrics**  
  - Per-trade returns compounding into an equity curve.  
  - Sharpe ratio, max drawdown, win rate, average PnL, etc.  
  - Breakdown for **long**, **short**, and **overall**.

---

## 📦 Installation

1. **Prerequisites**  
   - Rust (≥1.60) & Cargo  
   - Python (≥3.8) & virtualenv  
   - [`maturin`](https://github.com/PyO3/maturin) (`pip install maturin`)

2. **Build & install**  
   ```bash
   git clone https://github.com/Saransh-28/backtester.git
   cd backtester
   python3 -m venv .venv
   source .venv/bin/activate
   pip install maturin
   maturin develop --release
