# RustRedis

RustRedis is an experimental in-memory Redis-compatible key-value server written in Rust.
The project studies one research problem: how observability instrumentation (per-command
metrics collection) affects throughput, tail latency, and stability under concurrent load,
and how much that answer depends on benchmark design itself.

## TL;DR

This project measures the throughput cost of six per-command metrics strategies in a
Rust Redis-compatible server using **Randomized Multiple Interleaved Trials (RMIT)** —
a technique proposed by Abedi, Heard & Brecht (2015) and shown necessary for cloud
environments by
[Abedi & Brecht, ICPE 2017](docs/paper_draft.md#1-motivation-and-related-work), not
invented by this project — on two dedicated Azure VM configurations: a single
`Standard_D4s_v6` run, and `Standard_D8s_v6` run **eleven independent times** (ten
separately-provisioned VMs across three Azure regions, plus the original validation
run, pooled together) — **25,200 runs total**, 192 configurations, 100–3000 concurrent
clients, three workload mixes. No two-state throughput pattern appears (0 of 192
configurations flagged). Every strategy costs a small, consistent overhead — **at most
~2.3%** at any concurrency level (workload-averaged; single concurrency-by-workload
cells reach 2.5%) — with ThreadLocal, Sharded-2key, and GlobalMutex each pairwise
resolved in that order (sign-test p≈0.001 for each adjacent pair across all 11 runs),
and Sharded-N resolved as costliest (p≈0.012). The one gap that pooling did **not**
resolve is GlobalMutex vs. HdrHistogram — it got *weaker*, not stronger, as more runs
were added (p≈0.11 with 10 runs pooled → p≈0.23 with 11), a useful reminder that more
data doesn't guarantee every comparison resolves in the expected direction.

Full write-up, including related work, the replication methodology, and what this
paper does and doesn't newly contribute: [docs/paper_draft.md](docs/paper_draft.md).

## What is being compared

RustRedis has six interchangeable command-metrics collection strategies:

| Strategy | Description |
|---|---|
| `Disabled` | No command-level telemetry in the hot path (baseline). |
| `GlobalMutex` | One global lock protects all command counters. |
| `Sharded-2key` | Concurrent map keyed by command name (two entries in practice: GET and SET). |
| `Sharded-N` | Concurrent map keyed by the full logical (data) key — one entry per distinct key. |
| `ThreadLocal` | Per-thread accumulation with periodic flush. |
| `HdrHistogram` | Per-command latency histograms via the `hdrhistogram` crate. |

Each strategy runs the same workload and client counts, so any throughput/latency
difference isolates observability overhead and contention behavior rather than
something else changing between runs.

## RMIT results (current)

Two VM configurations, same design, different VM size and replication depth (a third,
laptop dataset, `experiment_results_rmit/`, remains in the repository but is not part
of the paper):

| Dataset | Hardware | Matrix | Runs | Notes |
|---|---|---|---:|---|
| [experiment_results_rmit_azure](experiment_results_rmit_azure) | Azure `Standard_D4s_v6` (4 vCPU, 16GB) | 6 strategies × 8 concurrency (100–1000) | 1,440 | Single run — VM dedicated solely to this benchmark |
| [experiment_results_adv_pooled](experiment_results_adv_pooled) | Azure `Standard_D8s_v6` (8 vCPU, 32GB) | 6 strategies × 8 concurrency (100–3000) × 3 workloads | 23,760 | **11 independent runs pooled** (2,160 each): [experiment_results_rmit_advanced](experiment_results_rmit_advanced) (original validation run) + [experiment_results_adv_iter_01](experiment_results_adv_iter_01) .. [_10](experiment_results_adv_iter_10) (10 more, on 4 VMs across 3 Azure regions) |

`benchmarks/combine_iterations.py` builds the pooled dataset from its 11 constituent
runs (offsetting each run's `block_id` so paired-block comparisons never mix runs from
different VMs — see paper §4c).

### No two-state pattern

The RMIT two-state detector (`benchmarks/analyze_rmit_results.py`) found **0 flagged
configurations across both datasets** (0 of 48 on Azure D4s_v6, 0 of 144 on the
11-run-pooled Azure D8s_v6 dataset — 192 configurations total). The detector is a simple
largest-gap heuristic, so this rules out large, well-separated state splits but not
subtler multimodality (paper §5).

### Run-to-run variability

Relative 95% bootstrap CI width on throughput (CI width ÷ median):

| | Azure D4s_v6 | Azure D8s_v6 (11 pooled) |
|---|---:|---:|
| Mean | 0.0091 | 0.0194 |
| Max | 0.0175 | 0.0552 |

This particular statistic is *wider* on the pooled D8s_v6 dataset than on a single
D8s_v6 run (0.0177) — pooling more runs does not automatically tighten every measure of
spread; see "How much does pooling really help?" below and paper §4c.

### Instrumentation overhead: small and consistent, at most ~2.3% (workload-averaged)

Mean throughput overhead vs. `disabled`, paired within the same RMIT repetition block
(the valid RMIT comparison — every strategy in a block saw the same machine-state
conditions):

| Strategy | Azure D4s_v6 | Azure D8s_v6 (11 pooled) |
|---|---:|---:|
| ThreadLocal | 0.56% | 0.60% |
| Sharded-2key | 0.85% | 0.90% |
| GlobalMutex | 0.99% | 1.43% |
| HdrHistogram | 1.42% | 1.47% |
| Sharded-N | 1.60% | 1.77% |

`ThreadLocal` is cheapest, `Sharded-2key` second, and `GlobalMutex` third on both VM
configurations — all three pairwise gaps hold in **11 of 11** individual D8s_v6 runs
(sign-test p≈0.001 each). `Sharded-N` is costliest, holding in 10 of 11 runs (p≈0.012).
`GlobalMutex` vs. `HdrHistogram` (ranks 3–4) is the one gap that stays unresolved: it
agrees in direction in only 8 of 11 runs (p≈0.23) and the point-estimate gap (0.04
percentage points) is below this design's own detectable-effect floor — see the paper's
§4b/§4c/§7 for the full statistical treatment. No strategy exceeds ~2.3% overhead at
any concurrency level (workload-averaged; single concurrency-by-workload cells reach
2.5%), and the ranking never crosses over in a way that holds up against measurement
noise (see below).

### Workload type doesn't change which strategy is cheapest

The advanced design adds read-heavy (80/20) and write-heavy (20/80) workloads alongside
the standard 50/50 mixed workload. ThreadLocal is cheapest and Sharded-2key
second-cheapest in all three; GlobalMutex and HdrHistogram trade the rank-3/4 spot
depending on workload (their gap is small and unresolved — see above), and Sharded-N is
most expensive overall:

| Strategy | Mixed | Read-heavy | Write-heavy |
|---|---:|---:|---:|
| ThreadLocal | 0.52% | 0.63% | 0.64% |
| Sharded-2key | 0.86% | 0.95% | 0.89% |
| GlobalMutex | 1.37% | 1.50% | 1.43% |
| HdrHistogram | 1.49% | 1.56% | 1.38% |
| Sharded-N | 1.73% | 1.87% | 1.72% |

### Graceful saturation, no cliff

On the pooled Azure D8s_v6 dataset (up to 3000 clients), the `disabled` baseline (mixed
workload) degrades smoothly rather than collapsing:

| Concurrency | Throughput (median) | p99 latency (median) |
|---:|---:|---:|
| 100 | ~290,000 ops/sec | 0.76 ms |
| 3000 | ~258,000 ops/sec | 79.4 ms |

That's roughly an 11% throughput decline against a 30× increase in concurrent clients —
the server is saturating gracefully, not falling over.

### On the "crossovers"

`analyze_rmit_results.py` also detects when the strategy with the highest median
throughput changes across concurrency levels. A single D8s_v6 run found 6 such
"crossovers"; pooling 10 runs found 4; **pooling all 11 finds 0.** Since every strategy
sits within ~2.3% of `disabled` at every concurrency level (workload-averaged), the
crossovers seen in smaller samples were sub-2% leader swaps — exactly what measurement
noise looks like. The count falling monotonically as more independent data was added
(6 → 4 → 0), rather than stabilizing on a nonzero number, is itself evidence the
earlier crossovers were noise: a real crossover would be expected to survive more data.

### How much does pooling really help?

Not uniformly. The paired-ratio comparisons above (which cancel out shared per-block
conditions before computing spread) tighten substantially with pooling — the
detectable-effect floor drops from ~2.1% for one D8s_v6 run to ~0.6–0.75% for the
11-run pool, roughly a 3x improvement, computed three ways (closed-form, a
cluster-aware bound treating each run as one data point rather than each of its 15
blocks, and an injected-effect simulation) that agree closely. But the simpler
CI-width statistic above did not tighten — it widened — because it reflects genuine
run-to-run variance that additional within-run samples can't shrink away, and the
GlobalMutex-vs-HdrHistogram ranking gap got statistically *weaker*, not stronger, with
the 11th run. See [docs/paper_draft.md §4c](docs/paper_draft.md#4c-how-much-replication-helps-and-how-to-account-for-it-correctly)
for the full treatment, including why this happens and how to check for it.

### A methodological pitfall worth knowing

The first attempt at the advanced dataset silently reported exactly `0 ops/sec` for
every run above ~1000 concurrent clients — no crash, no error, a validly-formed result.
Cause: the default open-file-descriptor limit (1024) on a fresh VM was far below the
concurrency being tested, so every client connection failed silently. Fixed in
`benchmarks/run_rmit_experiment.py` (`raise_fd_limit()`); see
[docs/paper_draft.md](docs/paper_draft.md) §3 for the full story.

### Caveats

- Both VM configurations are the same Azure VM family and x86-64 OS image, so they
  replicate the comparison across VM size, region (the D8s_v6 pool spans three Azure
  regions), and workload coverage, but not across clouds or hardware families.
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
python3 benchmarks/run_rmit_experiment.py --output-dir experiment_results_rmit_local
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit_local/raw_data_rmit.csv
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

## Legacy: v5/v12 (Apple M2, fixed order — superseded, not part of the paper)

Kept in the repository for provenance only; the paper and the results above use only the
Azure VM RMIT datasets. v5 ran on Apple M2 hardware with only 4 of the
current 6 strategies (`Sharded-2key`, `Sharded-N`, and `HdrHistogram` were added later).

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
**0.176**, max **0.698**. That dataset showed two throughput states that the fixed run
order confounded with the strategies, so it cannot rank them; it is not used in the paper.

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
