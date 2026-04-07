# MacOS M2 Experiment Protocol (2-core / 4-core / 8-core)

This protocol is designed to produce paper-usable results for RustRedis metrics-strategy experiments.

## 1) One-time setup (MacOS Apple Silicon)

```bash
# Xcode toolchain
xcode-select --install

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable

# redis-cli utility
brew install redis

# repo
git clone <your-fork-or-repo-url>
cd RustRedis
```

## 2) Build once

```bash
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
```

## 3) Run all required experiments

The script below runs all combinations:
- core settings: `2`, `4`, `8`
- metrics strategies: `global_mutex`, `sharded`, `thread_local`
- client concurrency: `100`, `500`, `1000`
- repetitions: `5` runs per configuration (default)

```bash
./benchmarks/run_macos_m2_research.sh
```

Optional overrides:

```bash
RUNS=5 REQUESTS_PER_CLIENT=10000 WORKLOAD=mixed KEY_SPACE=10000 VALUE_SIZE=64 \
  ./benchmarks/run_macos_m2_research.sh
```

> Use the same workload, key space, and value size across all runs.

## 4) Generate a single table for analysis

```bash
python3 benchmarks/summarize_macos_m2_results.py \
  --input results/macos_m2/<timestamp> \
  --output results/macos_m2/<timestamp>/summary.csv
```

## 5) What to report (paper-ready structure)

For each core setup (2/4/8), include:

1. **Machine details**
   - Device model (MacBook Air/Pro + chip variant)
   - Logical/physical cores
   - RAM
   - macOS version
   - Rust version (`rustc --version`)

2. **Configurations tested**
   - `global_mutex`, `sharded`, `thread_local`
   - at `100`, `500`, `1000` clients

3. **Metrics per configuration**
   - Throughput (ops/sec), mean ± stddev
   - p50 latency (µs), mean ± stddev
   - p99 latency (µs), mean ± stddev
   - Contention % estimate (from `cmdstat_lock_wait_us / Σtotal_time_us`)

4. **Run count**
   - Minimum 5 runs/configuration (already default in script)

## 6) Recommended reporting template

```text
Machine: MacOS M2, <RAM>, Rust <version>
Core setup: <2|4|8>

Concurrency: 100
GlobalMutex: throughput <mean±std>, p50 <mean±std>, p99 <mean±std>, contention <x%>
Sharded: ...
ThreadLocal: ...

Concurrency: 500
...

Concurrency: 1000
...
```

## 7) Notes on interpretation

Focus on scaling pattern, not absolute machine speed:
- Does `global_mutex` degrade as core count and clients rise?
- Does `sharded` scale partially?
- Does `thread_local` keep contention near zero?
