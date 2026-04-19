# RustRedis

RustRedis is an experimental in-memory Redis-compatible key-value server written in Rust.  
The project is focused on one research problem: how observability instrumentation affects throughput, tail latency, and stability under high concurrency.

The current canonical benchmark snapshot is based on [reports/final_experiment_v5.md](reports/final_experiment_v5.md) and artifacts under [results/final_experiment_v5/20260418_200904](results/final_experiment_v5/20260418_200904).

## Current Experiment Snapshot (v5)

| Parameter | Value |
|---|---|
| Run directory | `results/final_experiment_v5/20260418_200904` |
| Timestamp | `2026-04-18T20:09:04+05:30` |
| Host | `Sakshams-MacBook-Pro.local` |
| CPU | `Apple M2` |
| Logical / Physical CPU | `8 / 8` |
| Memory | `8589934592` bytes (8 GiB) |
| OS | `macOS 26.4 (25E246)` |
| Rust toolchain | `rustc 1.94.1`, `cargo 1.94.1` |
| Strategies | `Disabled`, `GlobalMutex`, `Sharded`, `ThreadLocal` |
| Shard count | `64` |
| ThreadLocal flush | `1000 records or 100ms` |
| Workload | Mixed (`50% GET / 50% SET`) |
| Runs per configuration | `30` |
| Concurrency levels | `100`, `500`, `1000` |
| Requests per client | `1000` |
| Total requests per config | `100000`, `500000`, `1000000` |
| Reproducible runner command (report) | `./benchmarks/run_final_experiment_v5.sh` |
| Server restart per config | `true` |
| Waits | `3s` before benchmark, `3s` between runs, `5s` between strategies |

## What Is Being Compared

RustRedis has interchangeable command-metrics collection strategies:

- `Disabled`: no command-level telemetry in the hot path.
- `GlobalMutex`: one global lock protects all command counters.
- `Sharded`: counters are distributed across shards.
- `ThreadLocal`: per-thread accumulation with periodic flush.

Each strategy is benchmarked on the same workload and client counts to isolate observability overhead and contention behavior.

## v5 Aggregated Results (30 successful runs each)

Values below are copied from [reports/final_experiment_v5.md](reports/final_experiment_v5.md), Section 3 and Section 4.

| Strategy | Clients | Throughput Mean (ops/sec) | Throughput CV | p99 Mean (us) | p99 CV | Stability (Throughput / p99) |
|---|---:|---:|---:|---:|---:|---|
| Disabled | 100 | 36611.528644 | 0.234482 | 28714.766667 | 0.209812 | Moderate / Moderate |
| Disabled | 500 | 147143.976597 | 0.025741 | 8900.200000 | 0.048309 | Stable / Stable |
| Disabled | 1000 | 47091.368160 | 0.739197 | 226996.700000 | 0.321273 | Unstable / Unstable |
| GlobalMutex | 100 | 27764.705092 | 0.922663 | 27128.833333 | 0.213708 | Unstable / Moderate |
| GlobalMutex | 500 | 31921.851325 | 0.115517 | 132980.733333 | 0.039584 | Moderate / Stable |
| GlobalMutex | 1000 | 32038.433293 | 0.061356 | 260195.233333 | 0.032253 | Stable / Stable |
| Sharded | 100 | 43980.181617 | 0.697623 | 22872.366667 | 0.378049 | Unstable / Unstable |
| Sharded | 500 | 35647.156977 | 0.615147 | 127441.100000 | 0.181130 | Unstable / Moderate |
| Sharded | 1000 | 31895.775837 | 0.045936 | 255582.500000 | 0.039344 | Stable / Stable |
| ThreadLocal | 100 | 36859.122201 | 0.976070 | 25817.466667 | 0.343201 | Unstable / Unstable |
| ThreadLocal | 500 | 148949.743028 | 0.008433 | 8586.666667 | 0.015320 | Stable / Stable |
| ThreadLocal | 1000 | 28567.854655 | 0.079255 | 269401.000000 | 0.045880 | Stable / Stable |

## Key Observations From v5

- At `500` clients, `ThreadLocal` is best on both means: `148949.743028 ops/sec` throughput and `8586.666667 us` p99.
- At `1000` clients, `Disabled` has the highest mean throughput (`47091.368160`), but this configuration is explicitly labeled unstable (high CV and outlier-heavy distribution).
- `GlobalMutex` and `Sharded` at `1000` clients are slower than `Disabled` but statistically much steadier (throughput CV near `0.05-0.06`).
- At `100` clients, all enabled telemetry strategies show unstable throughput due to large outliers.
- Reported critical anomaly checks:
  - `ThreadLocal p99 < Disabled p99` is confirmed at `100` and `500` clients.
  - `ThreadLocal p99 < Disabled p99` is not confirmed at `1000` clients.

## Experiment Integrity (as reported)

From [reports/final_experiment_v5.md](reports/final_experiment_v5.md), Section 6:

- Baseline avg CV: throughput `0.383538`, p99 `0.852380`
- Current avg CV: throughput `0.376785`, p99 `0.155655`
- Baseline avg relative CI width: throughput `0.212622`, p99 `0.472536`
- Current avg relative CI width: throughput `0.208879`, p99 `0.086291`
- Increasing requests improved stability and tightened CIs: `YES`
- Are results now reliable?: `NO`

Note: the report's CI line states `mean +/- (1.96 * stddev / sqrt(50))` even though sample size is `30`; this README keeps values exactly as reported.

## Quick Start

### 1. Build

```bash
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
```

### 2. Start server

```bash
RUSTREDIS_METRICS_STRATEGY=sharded cargo run --release --bin server
```

### 3. Run benchmark (new run)

```bash
cargo run --release --manifest-path benchmarks/Cargo.toml -- \
  --host 127.0.0.1 \
  --port 6379 \
  --concurrency 100,500,1000 \
  --requests 1000 \
  --runs 30 \
  --workload mixed \
  --key-space 10000 \
  --value-size 64 \
  --output-dir results/manual_v5_like
```

### 4. Useful automation scripts in this repo

- [benchmarks/run_final_matrix.sh](benchmarks/run_final_matrix.sh)
- [benchmarks/run_macos_m2_research.sh](benchmarks/run_macos_m2_research.sh)
- [benchmarks/run_paper_final_experiment.sh](benchmarks/run_paper_final_experiment.sh)

## Data and Reports

- Reports index: [reports/README.md](reports/README.md)
- Canonical v5 full report: [reports/final_experiment_v5.md](reports/final_experiment_v5.md)
- Additional reports:
  - [reports/final_experiment_report_enhanced.md](reports/final_experiment_report_enhanced.md)
  - [reports/final_experiment_report.md](reports/final_experiment_report.md)
  - [reports/final_experiment_details.md](reports/final_experiment_details.md)
- Canonical figures: [figures/canonical](figures/canonical)
- Compact repository map: [repo_structure.md](repo_structure.md)
- Raw benchmark trees:
  - [results/final_experiment_v5](results/final_experiment_v5)
  - [results/final_experiment](results/final_experiment)
  - [results/final_matrix](results/final_matrix)

## Architecture Overview

High-level server pipeline:

1. TCP listener accepts client connections.
2. Tokio task per connection parses RESP frames.
3. Command executor operates on shared DB state.
4. Optional persistence appends to AOF.
5. Telemetry path updates command metrics according to selected strategy.

Core modules:

- [src/bin/server.rs](src/bin/server.rs): server entry point
- [src/cmd/mod.rs](src/cmd/mod.rs): command parsing/execution
- [src/db.rs](src/db.rs): mutex-backed DB
- [src/db_dashmap.rs](src/db_dashmap.rs): sharded DB backend
- [src/connection.rs](src/connection.rs): network I/O
- [src/frame.rs](src/frame.rs): RESP framing
- [src/persistence.rs](src/persistence.rs): AOF persistence
- [src/command_metrics.rs](src/command_metrics.rs): metrics strategies
- [src/metrics.rs](src/metrics.rs): process/system counters
- [src/pubsub.rs](src/pubsub.rs): pub/sub manager

## Docs

- [docs/README.md](docs/README.md)
- [docs/system-design.md](docs/system-design.md)
- [docs/failure-analysis.md](docs/failure-analysis.md)
- [docs/macos_m2_experiment_protocol.md](docs/macos_m2_experiment_protocol.md)
- [docs/legacy_docs_archive.md](docs/legacy_docs_archive.md)

## License

MIT.