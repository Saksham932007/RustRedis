# RustRedis

RustRedis is an experimental in-memory Redis-compatible key-value server written in Rust.
The project studies one research problem: how observability instrumentation (per-command
metrics collection) affects throughput, tail latency, and stability under concurrent load,
and how much that answer depends on benchmark design itself.

## TL;DR

An earlier fixed-order benchmark design (v5/v12) reported throughput coefficients of
variation up to **0.70** and unexplained bimodal "fast/slow" states. Redesigning the
experiment around **Randomized Multiple Interleaved Trials (RMIT)** — a technique
defined by [Abedi & Brecht, ICPE 2017](docs/paper_draft.md#1-motivation-and-related-work)
for exactly this class of problem, not invented by this project — shuffling run order
per repetition instead of grouping by strategy, and rerunning across three independent
machines (a laptop and two cloud VMs), **eliminated the bimodal pattern entirely** (0 of
240 configurations flagged as two-state, across all three datasets) and cut relative
variability by roughly an order of magnitude. Under the corrected design, every
instrumentation strategy costs a small, consistent throughput overhead — **never more
than 2.3%**, holding steady from 100 to 3000 concurrent clients, with the top and bottom
of the per-strategy ranking (thread_local cheapest, sharded_n tied-or-highest) holding
across mixed/read-heavy/write-heavy workloads.

Full write-up, including related work and what this paper does and doesn't newly
contribute: [docs/paper_draft.md](docs/paper_draft.md).

## What is being compared

RustRedis has six interchangeable command-metrics collection strategies:

| Strategy | Description |
|---|---|
| `Disabled` | No command-level telemetry in the hot path (baseline). |
| `GlobalMutex` | One global lock protects all command counters. |
| `Sharded-2key` | Counters sharded across a fixed small key set. |
| `Sharded-N` | Counters sharded across the full command-name key space. |
| `ThreadLocal` | Per-thread accumulation with periodic flush. |
| `HdrHistogram` | Per-command latency histograms via the `hdrhistogram` crate. |

Each strategy runs the same workload and client counts, so any throughput/latency
difference isolates observability overhead and contention behavior rather than
something else changing between runs.

## RMIT results (current)

Three datasets, same design, different hardware and scope:

| Dataset | Hardware | Matrix | Runs | Notes |
|---|---|---|---:|---|
| [experiment_results_rmit](experiment_results_rmit) | Intel i3-10110U laptop (2C/4T, 8GB) | 6 strategies × 8 concurrency (25–500) | 720 | A dev session ran alongside this one — see caveat below |
| [experiment_results_rmit_azure](experiment_results_rmit_azure) | Azure `Standard_D4s_v6` (4 vCPU, 16GB) | 6 strategies × 8 concurrency (100–1000) | 1,440 | First clean-room run — VM dedicated solely to this benchmark |
| [experiment_results_rmit_advanced](experiment_results_rmit_advanced) | Azure `Standard_D8s_v6` (8 vCPU, 32GB) | 6 strategies × 8 concurrency (100–3000) × 3 workloads | 2,160 | Adds read-heavy/write-heavy workloads and a 3× wider concurrency range |

### No bimodal states, anywhere

The original v5/v12 runs reported an unexplained split between "fast" and "slow"
throughput states at several configurations. The RMIT redesign's bimodal detector
(`benchmarks/analyze_rmit_results.py`) found **0 flagged configurations across all
three datasets** (0 of 48 on the laptop, 0 of 48 on Azure D4s_v6, 0 of 144 on Azure
D8s_v6 — 240 configurations total) — the pattern did not reproduce once run order
stopped being confounded with time.

### Variability dropped by roughly an order of magnitude

Relative 95% bootstrap CI width on throughput (CI width ÷ median):

| | v12 (flawed design) | RMIT laptop | RMIT Azure D4s_v6 | RMIT Azure D8s_v6 (advanced) |
|---|---:|---:|---:|---:|
| Mean | 0.176 (as CV) | 0.0129 | 0.0091 | 0.0177 |
| Max | 0.698 (as CV) | 0.0386 | 0.0175 | 0.0420 |

(v12's column is CV = stddev/mean, not CI width — not the identical statistic, but both
measure spread relative to center, and the gap is large enough for the comparison to be
meaningful regardless.)

### Instrumentation overhead: small, consistent, never above 2.3%

Mean throughput overhead vs. `disabled`, paired within the same RMIT repetition block
(the valid RMIT comparison — every strategy in a block saw the same machine-state
conditions):

| Strategy | Laptop | Azure D4s_v6 | Azure D8s_v6 (advanced) |
|---|---:|---:|---:|
| ThreadLocal | 0.99% | 0.56% | 0.52% |
| Sharded-2key | 1.15% | 0.85% | 0.94% |
| GlobalMutex | 1.36% | 0.99% | 1.64% |
| HdrHistogram | 2.14% | 1.42% | 1.53% |
| Sharded-N | 1.97% | 1.60% | 1.98% |

`ThreadLocal` is the cheapest strategy on every machine; `Sharded-N` and `HdrHistogram`
are consistently the most expensive. No strategy ever exceeds ~2.3% overhead at any
concurrency level tested (100–3000), and the ranking never crosses over in a way that
holds up against measurement noise (see below).

### Workload type doesn't change which strategy is cheapest

The advanced dataset adds read-heavy (80/20) and write-heavy (20/80) workloads
alongside the standard 50/50 mixed workload. ThreadLocal is cheapest and Sharded-2key
second-cheapest in all three; Sharded-N is clearly most expensive in read-heavy and
write-heavy, and is a statistical tie with GlobalMutex for most expensive in mixed
(1.73% vs. 1.74% — below this design's minimum detectable effect, see
[docs/paper_draft.md §7](docs/paper_draft.md#7-limitations)):

| Strategy | Mixed | Read-heavy | Write-heavy |
|---|---:|---:|---:|
| ThreadLocal | 0.44% | 0.46% | 0.64% |
| Sharded-2key | 1.11% | 0.87% | 0.83% |
| HdrHistogram | 1.45% | 1.78% | 1.36% |
| GlobalMutex | 1.74% | 1.40% | 1.79% |
| Sharded-N | 1.73% | 2.15% | 2.07% |

### Graceful saturation, no cliff

On the advanced (D8s_v6, up to 3000 clients) dataset, the `disabled` baseline (mixed
workload) degrades smoothly rather than collapsing:

| Concurrency | Throughput (median) | p99 latency (median) |
|---:|---:|---:|
| 100 | ~280,000 ops/sec | 0.8 ms |
| 3000 | ~235,000 ops/sec | 85.5 ms |

That's a ~16% throughput decline against a 30× increase in concurrent clients — the
server is saturating gracefully, not falling over.

### On the "crossovers"

`analyze_rmit_results.py` also detects when the strategy with the highest median
throughput changes across concurrency levels. It found 6 such "crossovers" in the
advanced dataset — but since every strategy sits within ~2.3% of `disabled` at every
concurrency level, a leader change driven by sub-2% differences is exactly what
measurement noise looks like, not a real strategy-concurrency interaction. We report the
number because the tooling can now detect a genuine crossover if one exists, but the
honest reading of this dataset is: no meaningful crossover, overhead is flat and small
throughout.

### A methodological pitfall worth knowing

The first attempt at the advanced dataset silently reported exactly `0 ops/sec` for
every run above ~1000 concurrent clients — no crash, no error, a validly-formed result.
Cause: the default open-file-descriptor limit (1024) on a fresh VM was far below the
concurrency being tested, so every client connection failed silently. Fixed in
`benchmarks/run_rmit_experiment.py` (`raise_fd_limit()`); see
[docs/paper_draft.md](docs/paper_draft.md) §3 for the full story.

### Caveats

- The laptop run had an active development session sharing its 4 threads throughout
  (see `experiment_results_rmit/metadata_rmit.json`); the Azure runs don't have this
  confound and should be treated as primary where they disagree (in practice they agree
  closely).
- Client and server always share the same machine — at the highest concurrency levels
  this is a genuine thread-oversubscription stress test, not an isolated server
  measurement.
- Only x86-64 hardware tested (4–8 vCPUs/threads); no claim about other architectures or
  core counts.

Full detail, methodology, and remaining open items: **[docs/paper_draft.md](docs/paper_draft.md)**.

## Quick start

### 1. Build

```bash
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
```

### 2. Run the RMIT experiment

```bash
python3 benchmarks/run_rmit_experiment.py --output-dir experiment_results_rmit
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit/raw_data_rmit.csv
```

Read [docs/rmit_experiment_protocol.md](docs/rmit_experiment_protocol.md) first — it has
a manual machine-control checklist (power, sleep, background processes) that materially
affects result quality, plus the cloud-VM path (`--workloads mixed,read-heavy,write-heavy`
for the 3-D design used in the advanced dataset above).

### 3. Start a server manually / run a one-off benchmark

```bash
RUSTREDIS_METRICS_STRATEGY=sharded_n cargo run --release --bin server
```

```bash
cargo run --release --manifest-path benchmarks/Cargo.toml -- \
  --host 127.0.0.1 --port 6379 \
  --concurrency 100,500,1000 --requests 1000 --runs 30 \
  --workload mixed --key-space 10000 --value-size 64 \
  --output-dir results/manual_run
```

## Documentation

- [docs/paper_draft.md](docs/paper_draft.md): full write-up with all three RMIT datasets
- [docs/rmit_experiment_protocol.md](docs/rmit_experiment_protocol.md): current experiment protocol (laptop + cloud-VM paths)
- [docs/system-design.md](docs/system-design.md)
- [docs/failure-analysis.md](docs/failure-analysis.md)
- [repo_structure.md](repo_structure.md): compact repository map
- [docs/macos_m2_experiment_protocol.md](docs/macos_m2_experiment_protocol.md): superseded, kept for provenance (M2 hardware)
- [docs/legacy_docs_archive.md](docs/legacy_docs_archive.md)

## Legacy: v5/v12 (superseded fixed-order design)

Kept for provenance and as the "before" side of the before/after comparison above — not
the current canonical result. v5 ran on Apple M2 hardware with only 4 of the current 6
strategies (`Sharded-2key`, `Sharded-N`, and `HdrHistogram` were added later).

| Strategy | Clients | Throughput Mean (ops/sec) | Throughput CV | p99 Mean (us) |
|---|---:|---:|---:|---:|
| Disabled | 100 | 36,612 | 0.234 | 28,715 |
| Disabled | 500 | 147,144 | 0.026 | 8,900 |
| Disabled | 1000 | 47,091 | 0.739 | 226,997 |
| GlobalMutex | 1000 | 32,038 | 0.061 | 260,195 |
| Sharded | 1000 | 31,896 | 0.046 | 255,583 |
| ThreadLocal | 500 | 148,950 | 0.008 | 8,587 |
| ThreadLocal | 1000 | 28,568 | 0.079 | 269,401 |

v12 (30 repetitions, fixed order, Apple M2) mean throughput CV across 48 configurations:
**0.176**, max **0.698** — see [docs/paper_draft.md](docs/paper_draft.md) §2 for the full
evidence this instability was a benchmark-design artifact, not a server property.

- Full v5 report: [reports/final_experiment_v5.md](reports/final_experiment_v5.md)
- Additional reports: [reports/final_experiment_report_enhanced.md](reports/final_experiment_report_enhanced.md), [reports/final_experiment_report.md](reports/final_experiment_report.md), [reports/final_experiment_details.md](reports/final_experiment_details.md)
- v12 dataset: [experiment_results_v12](experiment_results_v12)
- Canonical figures: [figures/canonical](figures/canonical)
- Legacy raw benchmark trees (pre-v12): [results/final_experiment](results/final_experiment), [results/final_matrix](results/final_matrix), [results/macos_m2](results/macos_m2), [results/metrics_strategy_mandatory](results/metrics_strategy_mandatory), [results/system_validation_v15](results/system_validation_v15)
- Legacy automation scripts (fixed-order runner, superseded): [benchmarks/run_final_matrix.sh](benchmarks/run_final_matrix.sh), [benchmarks/run_macos_m2_research.sh](benchmarks/run_macos_m2_research.sh), [benchmarks/run_paper_final_experiment.sh](benchmarks/run_paper_final_experiment.sh), [benchmarks/run_final_experiment_v12.py](benchmarks/run_final_experiment_v12.py)

## Architecture overview

1. TCP listener accepts client connections.
2. Tokio task per connection parses RESP frames.
3. Command executor operates on shared DB state.
4. Optional persistence appends to AOF.
5. Telemetry path updates command metrics according to the selected strategy.

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

## License

MIT.
