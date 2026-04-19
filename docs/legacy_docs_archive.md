# Legacy Docs Archive

This file consolidates older documentation that was previously split across multiple files in the docs directory.

Archived source files:
- docs/system_design_and_theory.md
- docs/scalable_metrics.md
- docs/research_data.md
- docs/metrics_strategy_mandatory_results_20260408.md



---

## Source File: docs/system_design_and_theory.md

# system_design_and_theory.md

## 1. System Overview

RustRedis is a concurrent in-memory key-value server implemented in Rust, exposing a Redis-compatible command subset over RESP. The server is designed as a systems research platform for studying concurrency control, persistence tradeoffs, and telemetry overhead.

High-level architecture:

- TCP listener accepts client sockets.
- Each connection is handled by a dedicated Tokio task.
- Commands are parsed from RESP frames and executed by a command dispatcher.
- Shared state components include:
  - database storage
  - AOF persistence writer
  - pub/sub channel registry
  - global server counters and per-command telemetry

Async model:

- Tokio runtime uses `multi_thread` flavor.
- Work is scheduled across worker threads with task-level concurrency.
- Network handling and command execution are asynchronous at the task level.

Shared state model:

- Database state in the default path is guarded by `Arc<Mutex<...>>`.
- AOF file handle is guarded by `Arc<Mutex<File>>`.
- Server-wide counters use atomic integers.
- Per-command observability uses a strategy-selectable collector (`disabled`, `global_mutex`, `sharded`, `thread_local_batched`).

## 2. Observability System Design

RustRedis implements per-command telemetry inspired by database statement statistics systems. For each command type (for example GET, SET, HGET), the collector tracks:

- `calls`
- `total_time_us`
- `min_time_us`
- `max_time_us`
- derived `avg_time_us`

Hot-path instrumentation:

- Command execution is timed around the command dispatcher.
- The measured duration is recorded by the active metrics strategy.
- The same request path also updates server-level totals.

Query path:

- `CMDSTAT` returns a snapshot view of per-command metrics.
- For `global_mutex`, `CMDSTAT` also exposes `cmdstat_lock_wait_us`.

Why shared state is required:

- Connection tasks execute concurrently and must contribute to a single global telemetry view.
- `CMDSTAT` requires globally visible aggregates, not per-task private counters.
- Correct aggregation under concurrency requires explicit synchronization or delayed merge mechanisms.

## 3. Metrics Collection Strategies

### 3.1 Global Mutex

Design:

- One global `Mutex<HashMap<command, CommandStat>>` protects all command counters.

Synchronization model:

- Every telemetry update acquires the same mutex.
- Lock-wait time is explicitly measured and accumulated.

Lock convoy mechanism:

- When many worker threads attempt updates, wait queues form around one lock.
- Threads repeatedly block and wake on the same synchronization point.

Expected behavior:

- Simple semantics and immediate consistency.
- Increasing contention and lock wait under rising request concurrency.
- Tail latency sensitivity when lock wait propagates into request completion time.

### 3.2 Sharded (DashMap)

Design:

- A sharded concurrent map stores command statistics.
- Commands map to shards via hashing.

Synchronization model:

- Updates on different shards proceed in parallel.
- Updates mapping to the same shard still serialize.

Reduced contention mechanism:

- Independent command keys can avoid single-lock serialization.
- Cache-line and lock ownership pressure are distributed across shards.

Limitations:

- Same-key hot spots still contend.
- Skewed command distributions can create shard imbalance.
- Consistency remains immediate for each update, so synchronization is still on the hot path.

### 3.3 Thread-Local Batching

Design:

- Each worker thread stores command stats in thread-local maps.
- Hot-path updates are local writes without shared lock acquisition.

Batching and flush:

- Local stats are pushed to a global pending queue periodically (threshold-based trigger).
- A background flush task drains pending batches at a fixed interval (100 ms) and merges into a global snapshot.

Coordination mechanisms:

- Global `records_since_flush` atomic counter for trigger heuristics.
- Global `pending_batches` mutex-protected queue.
- Global `global_snapshot` mutex for merge output.

Eventual consistency:

- `CMDSTAT` is not strictly real-time across all workers.
- Visibility depends on when each thread pushes local state and when the flusher drains pending batches.

### 3.4 Disabled (Baseline)

Design:

- Per-command telemetry recording is bypassed.

Role in analysis:

- Defines the reference performance envelope without observability cost.
- Used to compute throughput overhead and p99 deltas for other strategies.

## 4. Concurrency and Contention Theory

Mutex contention:

- Let lock request arrival rate be `lambda` and average lock hold time be `s`.
- Lock utilization can be approximated by `rho = lambda * s`.
- As `rho` approaches 1, queueing delay rises nonlinearly.

Lock convoy effect:

- A single shared lock in a high-arrival-rate system creates serialized access.
- Waiting threads accumulate, increasing latency variance and reducing useful CPU work per unit time.

Scaling with thread count:

- More worker threads increase potential overlap in lock acquisition attempts.
- If telemetry is on the request critical path, lock pressure grows with request concurrency.

Shared-state bottlenecks:

- Any globally shared mutable structure can become a serialization boundary.
- The bottleneck may be explicit (mutex) or implicit (global merge queue, atomic coordination, scheduler handoff).

## 5. Why Performance Changes with Concurrency

Queueing delays:

- Requests queue at multiple points: socket I/O, runtime scheduling, and shared-state synchronization.
- Tail latency expands when any queue becomes saturated.

Increased contention:

- Concurrent command completion means concurrent telemetry updates.
- Strategies with centralized synchronization experience higher wait under load.

Scheduling pressure:

- High connection counts increase runnable task population.
- Context switching, wakeups, and cache invalidation amplify overhead.
- Under pressure, timing jitter increases and run-to-run variance grows.

## 6. Thread-Local Failure Analysis

Implementation-specific mechanics:

- Thread-local maps are private to each worker thread.
- The background flusher drains only globally queued batches.
- A worker's local map is merged only after that worker pushes a batch.

Failure-prone coordination pattern:

- Flush triggering is globally counted (`records_since_flush`) but local-to-thread data movement is not globally forced.
- This can produce uneven draining where some worker-local data lags behind others.

Shared coordination remains present:

- `pending_batches` and `global_snapshot` are still shared synchronization points.
- Under heavy update rates, merge and queue coordination can become an additional bottleneck.

Eventual-consistency side effects:

- Observability freshness depends on flush cadence and worker participation.
- During overload, delayed or uneven flush can widen telemetry lag.

Memory and scheduling pressure:

- Batched aggregation adds transient in-memory structures and merge work.
- Under high concurrency, this adds work to a scheduler that is already saturated by request handling.

Observed instability profile explained by mechanism:

- Request errors can accumulate without process crash if the server remains alive but cannot service requests within timeout windows.
- This matches a liveness degradation pattern rather than a fail-stop panic pattern.

## 7. Tradeoff Analysis

Synchronization vs performance:

- Strong immediate consistency in telemetry generally requires more synchronization on the hot path.
- Reducing synchronization can improve hot-path cost but introduces aggregation complexity.

Contention vs stability:

- Centralized locking has predictable semantics but can bottleneck under load.
- Deferred aggregation reduces direct lock contention but can introduce coordination instability.

No universally best strategy:

- Optimal choice depends on workload shape, command skew, concurrency level, and required telemetry freshness.
- Strategy selection is a systems tradeoff, not a one-time static winner.

## 8. Real-World Mapping

PostgreSQL `pg_stat_statements` analogy:

- Both systems maintain per-statement/per-command shared telemetry.
- Both must balance measurement fidelity against hot-path overhead.

LWLock-style contention analogy:

- Global telemetry structures in database engines often use lock primitives.
- Under heavy parallel query traffic, lock contention on statistics structures can materially affect tail latency.

General database telemetry patterns:

- Sharding/partitioning counters to reduce collision domains.
- Per-worker local counters with periodic aggregation.
- Hybrid designs with bounded staleness for reduced overhead.

## 9. Design Implications

When to use each strategy:

- `disabled`: latency-critical environments where per-command telemetry is not required online.
- `global_mutex`: simple deployments, low to moderate concurrency, strict immediate consistency needs.
- `sharded`: default choice when balanced overhead and real-time visibility are both required.
- `thread_local_batched`: only with careful validation of flush/merge behavior and overload handling.

Production considerations:

- Treat telemetry as part of the critical path unless proven otherwise.
- Validate behavior at target concurrency, not only at low load.
- Monitor both throughput and variance, not throughput alone.
- Include explicit saturation and timeout observability for telemetry subsystems.

Observability design principles:

- Keep measurement cost proportional to available headroom.
- Minimize global serialization points.
- Prefer predictable degradation modes.
- Separate correctness of business logic from freshness of diagnostic data.


---

## Source File: docs/scalable_metrics.md

# Scalable Concurrent Metrics Collection

## Overview
RustRedis introduces a highly granular, `pg_stat_statements`-inspired metrics collection system designed to operate under severe high-contention workloads safely. Rather than relying on simple global aggregations, this feature tracks the exact execution profile of every individual command (e.g., total calls, cumulative time, min/max latencies) while avoiding standard concurrency bottlenecks.

Because fine-grained telemetry naturally introduces synchronization points in the hot execution path, RustRedis implements three distinct configurable metrics collection strategies. This allows operators and researchers to evaluate the fundamental concurrency tradeoff between telemetry granularity and execution throughput.

## Functional Components

### 1. The Global Metrics Collector
The core of the feature is the `CommandStats` tracker. For every command processed by the `Command Executor`, the following data points are recorded:
- **`calls`**: The absolute count of executions.
- **`total_time_us`**: The sum of all execution durations, measured via high-resolution `Instant::now()` timings.
- **`min_time_us`** / **`max_time_us`**: Boundary values for latency distribution analysis.

These statistics are queryable in real-time using the `CMDSTAT` RESP command (or an extended `STATS` payload), outputting parsed key-value summaries for each command type that has been executed at least once since boot.

### 2. Strategy A: Global Lock Baseline
The system includes a canonical baseline implementation: a single global `Mutex<HashMap<&'static str, CommandStat>>`. 
- **How it works:** Every Tokio task executing a command must acquire this central lock to update the metrics.
- **Why it matters:** At >1,000 concurrent clients, this design inevitably causes a "lock convoy," where threads spend more time waiting in a queue to update metrics than processing actual TCP non-blocking I/O. It serves as the baseline measurement for contention analysis.

### 3. Strategy B: Sharded Metrics System
To alleviate global contention, a sharded strategy partitions the lock space.
- **How it works:** Instead of one global lock, the metric store is partitioned (e.g., using `DashMap` or an array of `Mutex<CommandStat>`) using the command type as the hash key.
- **Why it matters:** Read-heavy workloads like `GET` commands will still contend on the same shard, but mixed workloads (`GET`, `SET`, `HGET`) distribute lock acquisitions across different cache lines, drastically lowering thread collision probabilities compared to Strategy A.

### 4. Strategy C: Thread-local Batched System
The ultimate lock-free optimization for high-throughput scaling.
- **How it works:** Using Thread-Local Storage (TLS) or Tokio's task-local macros, each worker thread manages its isolated `HashMap` of command statistics. Telemetry updates on the hot path require exactly zero synchronization barriers and no lock acquisitions.
- **The Sync Process:** At defined intervals (e.g., every 1,000 commands or periodically triggered by a background ticker), the worker thread asynchronously flushes its batches into a globally readable atomic snapshot. 
- **Why it matters:** This decouples telemetry recording from network I/O and command execution. It perfectly scales with CPU core count without creating multi-core latency spikes.

## Integration & Benchmarks
This feature is cleanly integrated into the `Command Executor` router logic in `src/cmd/mod.rs`. Before a command is dispatched, an `Instant::now()` timer begins; upon the command's successful return, the duration is forwarded to the active telemetry collector.

The `benchmarks` suite is explicitly expanded to support this feature. Users can run standard tests using `--metrics-strategy=sharded` vs `--metrics-strategy=thread-local` to empirically measure tail latency (p99) reductions under 1,000+ open connections. The system ensures that telemetry adds minimal overhead to the `EverySecond` AOF persistence mechanisms natively present in RustRedis.


---

## Source File: docs/research_data.md

# research_data.md

## 1. Overview

This document consolidates experimental artifacts, benchmark outputs, and structured measurements for the observability strategy study in RustRedis.

Research goal: **observability overhead under high concurrency**.

Primary dataset used for the full matrix:

- `results/final_matrix/20260408_154844/`

Additional datasets retained as supplementary evidence:

- `results/metrics_strategy_mandatory/20260408_010639/`
- `results/macos_m2/20260408_005543/`
- `results/final_matrix/20260408_154750/` (incomplete run)

## 2. Experimental Setup

### 2.1 Hardware

Primary matrix hardware metadata (`results/final_matrix/20260408_154844/machine_details.txt`):

- Host: `Sakshams-MacBook-Pro.local`
- CPU architecture: `arm64` (Darwin kernel `RELEASE_ARM64_T8112`)
- Logical CPUs: `8`
- Physical CPUs: `8`
- RAM: `8589934592` bytes (~8 GiB)
- OS: `macOS 26.4 (Build 25E246)`
- Kernel: `Darwin 25.4.0`
- Rust compiler: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Cargo: `cargo 1.94.1 (29ea6fb6a 2026-03-24)`

Runtime metadata from source:

- Tokio runtime flavor: `multi_thread` (`src/bin/server.rs`)
- Tokio crate version: `1.40` (`Cargo.toml`)

### 2.2 Workload

Primary matrix workload (`benchmarks/run_final_matrix.sh`, benchmark args, and matrix metadata):

- Workload type: `mixed`
- GET/SET ratio: `50% GET / 50% SET` (`benchmarks/src/main.rs`)
- Keyspace size: `10000`
- Value size: `64` bytes
- Concurrency levels: `100, 500, 1000`
- Core configurations: `4`, `8` (via `TOKIO_WORKER_THREADS`)
- Strategies: `disabled`, `global_mutex`, `sharded`, `thread_local`
- Runs per configuration: `3`

Request accounting details from benchmark implementation:

- CLI argument `--requests=10000` is divided by concurrency (`requests / conc`) per client thread.
- Effective total attempted operations per configuration are approximately `10000` (before errors).

### 2.3 Metrics Collected

From aggregated JSON and matrix summary files:

- Throughput: `ops_per_sec_mean`, `ops_per_sec_stddev`
- Latency: `p50_us_mean/stddev`, `p99_us_mean/stddev`
- Variance/stability: standard deviation and coefficient of variation (CV)
- Error counts: `total_errors` and per-run `errors`
- Overhead metrics (vs disabled baseline):
  - Throughput overhead percentage
  - p99 delta percentage
- Lock contention metrics (if available):
  - `cmdstat_lock_wait_us` (global mutex strategy only)
  - Derived contention estimate: `lock_wait_us / sum(total_time_us)`

## 3. Experiment Matrix

Primary matrix definition (`results/final_matrix/20260408_154844`):

Strategies:

- `disabled`
- `global_mutex`
- `sharded`
- `thread_local`

Core configurations:

- `4-core`
- `8-core`

Concurrency levels:

- `100`
- `500`
- `1000`

Total primary matrix cells:

- `2 cores x 4 strategies x 3 concurrencies = 24` configurations

## 4. Results (RAW + STRUCTURED)

### 4.1 Final Matrix Table

Source: `results/final_matrix/20260408_154844/observability_matrix_summary.csv` and `.md`

| Core   | Strategy     | Clients | Throughput mean +- stddev | p99 mean +- stddev (us) | Throughput CV | p99 CV | Errors | Throughput overhead vs disabled | p99 delta vs disabled |
| ------ | ------------ | ------: | ------------------------: | ----------------------: | ------------: | -----: | -----: | ------------------------------: | --------------------: |
| 4-core | disabled     |     100 |     143632.37 +- 12448.67 |       1741.67 +- 626.30 |         0.087 |  0.360 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |     100 |     138056.73 +- 25613.60 |       1727.33 +- 772.59 |         0.186 |  0.447 |      0 |                          +3.88% |                -0.82% |
| 4-core | sharded      |     100 |      90411.06 +- 39952.25 |      4952.67 +- 3063.43 |         0.442 |  0.619 |      0 |                         +37.05% |              +184.36% |
| 4-core | thread_local |     100 |      82123.91 +- 30970.99 |      9815.00 +- 9161.58 |         0.377 |  0.933 |      0 |                         +42.82% |              +463.54% |
| 4-core | disabled     |     500 |     125881.07 +- 45951.43 |       4109.00 +- 871.34 |         0.365 |  0.212 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |     500 |     114389.20 +- 34969.32 |       3441.00 +- 158.19 |         0.306 |  0.046 |      0 |                          +9.13% |               -16.26% |
| 4-core | sharded      |     500 |     106670.78 +- 57129.70 |      9307.67 +- 8193.05 |         0.536 |  0.880 |      0 |                         +15.26% |              +126.52% |
| 4-core | thread_local |     500 |      38272.14 +- 43719.03 |    38717.00 +- 63943.61 |         1.142 |  1.652 |  15960 |                         +69.60% |              +842.25% |
| 4-core | disabled     |    1000 |      128389.02 +- 1508.38 |       5285.00 +- 398.98 |         0.012 |  0.075 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |    1000 |     105088.07 +- 18144.92 |       5521.67 +- 820.80 |         0.173 |  0.149 |      0 |                         +18.15% |                +4.48% |
| 4-core | sharded      |    1000 |      91352.13 +- 35453.37 |    20179.67 +- 26596.99 |         0.388 |  1.318 |      0 |                         +28.85% |              +281.83% |
| 4-core | thread_local |    1000 |              0.00 +- 0.00 |            0.00 +- 0.00 |         0.000 |  0.000 |  30000 |                        +100.00% |              -100.00% |
| 8-core | disabled     |     100 |     136655.09 +- 14670.27 |       1920.00 +- 831.38 |         0.107 |  0.433 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |     100 |     135929.94 +- 21008.12 |       1676.67 +- 480.22 |         0.155 |  0.286 |      0 |                          +0.53% |               -12.67% |
| 8-core | sharded      |     100 |     132813.11 +- 24009.25 |       2216.67 +- 973.90 |         0.181 |  0.439 |      0 |                          +2.81% |               +15.45% |
| 8-core | thread_local |     100 |     133243.34 +- 19785.69 |       1920.67 +- 852.34 |         0.148 |  0.444 |      0 |                          +2.50% |                +0.03% |
| 8-core | disabled     |     500 |     136904.73 +- 15681.26 |      5399.00 +- 1054.67 |         0.115 |  0.195 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |     500 |     116518.53 +- 23648.43 |      6698.33 +- 4937.92 |         0.203 |  0.737 |      0 |                         +14.89% |               +24.07% |
| 8-core | sharded      |     500 |     126927.29 +- 32057.34 |       4306.67 +- 795.95 |         0.253 |  0.185 |      0 |                          +7.29% |               -20.23% |
| 8-core | thread_local |     500 |      17860.39 +- 17564.46 |    71060.00 +- 87990.31 |         0.983 |  1.238 |  15980 |                         +86.95% |             +1216.17% |
| 8-core | disabled     |    1000 |      97064.92 +- 39319.73 |       6293.67 +- 509.51 |         0.405 |  0.081 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |    1000 |     117447.35 +- 14885.43 |       5506.67 +- 503.51 |         0.127 |  0.091 |      0 |                         -21.00% |               -12.50% |
| 8-core | sharded      |    1000 |      103494.09 +- 9051.53 |      7680.67 +- 2505.74 |         0.087 |  0.326 |      0 |                          -6.62% |               +22.04% |
| 8-core | thread_local |    1000 |            11.26 +- 19.50 |   72800.00 +- 126093.30 |         1.732 |  1.732 |  29980 |                         +99.99% |             +1056.72% |

### 4.2 Key Graphs (described)

Observed/available graph artifact:

- `results/final_matrix/20260408_154844/observability_cost_vs_concurrency.png`
  - Contains 2x2 panels: throughput and p99 for 4-core and 8-core, each plotted across 100/500/1000 clients and four strategies.

Graph descriptions from structured data (`observability_matrix_summary.csv`):

- **Observability Cost vs Concurrency**
  - Throughput overhead relative to disabled is near zero to moderate for `global_mutex` and `sharded` in multiple cells.
  - `thread_local` shows high overhead and large error counts at `500` and `1000` clients.
- **Throughput Scaling**
  - Per-strategy throughput trajectories are provided by the table in 4.1 and can be plotted directly against client count.
- **Latency Scaling**
  - p99 trajectories are provided by the table in 4.1 and can be plotted directly against client count.

### 4.3 Per-Strategy Analysis (NO interpretation)

#### disabled

- 4-core:
  - c=100: throughput `143632.37 +- 12448.67`, p99 `1741.67 +- 626.30`, throughput CV `0.087`, p99 CV `0.360`, errors `0`
  - c=500: throughput `125881.07 +- 45951.43`, p99 `4109.00 +- 871.34`, throughput CV `0.365`, p99 CV `0.212`, errors `0`
  - c=1000: throughput `128389.02 +- 1508.38`, p99 `5285.00 +- 398.98`, throughput CV `0.012`, p99 CV `0.075`, errors `0`
- 8-core:
  - c=100: throughput `136655.09 +- 14670.27`, p99 `1920.00 +- 831.38`, throughput CV `0.107`, p99 CV `0.433`, errors `0`
  - c=500: throughput `136904.73 +- 15681.26`, p99 `5399.00 +- 1054.67`, throughput CV `0.115`, p99 CV `0.195`, errors `0`
  - c=1000: throughput `97064.92 +- 39319.73`, p99 `6293.67 +- 509.51`, throughput CV `0.405`, p99 CV `0.081`, errors `0`

#### global_mutex

- 4-core:
  - c=100: throughput `138056.73 +- 25613.60`, p99 `1727.33 +- 772.59`, throughput CV `0.186`, p99 CV `0.447`, errors `0`
  - c=500: throughput `114389.20 +- 34969.32`, p99 `3441.00 +- 158.19`, throughput CV `0.306`, p99 CV `0.046`, errors `0`
  - c=1000: throughput `105088.07 +- 18144.92`, p99 `5521.67 +- 820.80`, throughput CV `0.173`, p99 CV `0.149`, errors `0`
- 8-core:
  - c=100: throughput `135929.94 +- 21008.12`, p99 `1676.67 +- 480.22`, throughput CV `0.155`, p99 CV `0.286`, errors `0`
  - c=500: throughput `116518.53 +- 23648.43`, p99 `6698.33 +- 4937.92`, throughput CV `0.203`, p99 CV `0.737`, errors `0`
  - c=1000: throughput `117447.35 +- 14885.43`, p99 `5506.67 +- 503.51`, throughput CV `0.127`, p99 CV `0.091`, errors `0`

#### sharded

- 4-core:
  - c=100: throughput `90411.06 +- 39952.25`, p99 `4952.67 +- 3063.43`, throughput CV `0.442`, p99 CV `0.619`, errors `0`
  - c=500: throughput `106670.78 +- 57129.70`, p99 `9307.67 +- 8193.05`, throughput CV `0.536`, p99 CV `0.880`, errors `0`
  - c=1000: throughput `91352.13 +- 35453.37`, p99 `20179.67 +- 26596.99`, throughput CV `0.388`, p99 CV `1.318`, errors `0`
- 8-core:
  - c=100: throughput `132813.11 +- 24009.25`, p99 `2216.67 +- 973.90`, throughput CV `0.181`, p99 CV `0.439`, errors `0`
  - c=500: throughput `126927.29 +- 32057.34`, p99 `4306.67 +- 795.95`, throughput CV `0.253`, p99 CV `0.185`, errors `0`
  - c=1000: throughput `103494.09 +- 9051.53`, p99 `7680.67 +- 2505.74`, throughput CV `0.087`, p99 CV `0.326`, errors `0`

#### thread_local

- 4-core:
  - c=100: throughput `82123.91 +- 30970.99`, p99 `9815.00 +- 9161.58`, throughput CV `0.377`, p99 CV `0.933`, errors `0`
  - c=500: throughput `38272.14 +- 43719.03`, p99 `38717.00 +- 63943.61`, throughput CV `1.142`, p99 CV `1.652`, errors `15960`
  - c=1000: throughput `0.00 +- 0.00`, p99 `0.00 +- 0.00`, throughput CV `0.000`, p99 CV `0.000`, errors `30000`
- 8-core:
  - c=100: throughput `133243.34 +- 19785.69`, p99 `1920.67 +- 852.34`, throughput CV `0.148`, p99 CV `0.444`, errors `0`
  - c=500: throughput `17860.39 +- 17564.46`, p99 `71060.00 +- 87990.31`, throughput CV `0.983`, p99 CV `1.238`, errors `15980`
  - c=1000: throughput `11.26 +- 19.50`, p99 `72800.00 +- 126093.30`, throughput CV `1.732`, p99 CV `1.732`, errors `29980`

Raw per-run error evidence (thread_local):

- 4-core, c=500: per-run errors `[0, 5960, 10000]`, per-run total_ops `[10000, 4040, 0]`
- 4-core, c=1000: per-run errors `[10000, 10000, 10000]`, per-run total_ops `[0, 0, 0]`
- 8-core, c=500: per-run errors `[0, 5980, 10000]`, per-run total_ops `[10000, 4020, 0]`
- 8-core, c=1000: per-run errors `[10000, 9980, 10000]`, per-run total_ops `[0, 20, 0]`

### 4.4 Core Scaling Results

Computed from `observability_matrix_summary.csv` as `(8-core / 4-core - 1)`.

| Strategy     | Clients | Throughput delta (8c vs 4c) | p99 delta (8c vs 4c) | Throughput CV (4c -> 8c) | Errors (4c -> 8c) |
| ------------ | ------: | --------------------------: | -------------------: | -----------------------: | ----------------: |
| disabled     |     100 |                      -4.86% |              +10.24% |           0.087 -> 0.107 |            0 -> 0 |
| disabled     |     500 |                      +8.76% |              +31.39% |           0.365 -> 0.115 |            0 -> 0 |
| disabled     |    1000 |                     -24.40% |              +19.09% |           0.012 -> 0.405 |            0 -> 0 |
| global_mutex |     100 |                      -1.54% |               -2.93% |           0.186 -> 0.155 |            0 -> 0 |
| global_mutex |     500 |                      +1.86% |              +94.66% |           0.306 -> 0.203 |            0 -> 0 |
| global_mutex |    1000 |                     +11.76% |               -0.27% |           0.173 -> 0.127 |            0 -> 0 |
| sharded      |     100 |                     +46.90% |              -55.24% |           0.442 -> 0.181 |            0 -> 0 |
| sharded      |     500 |                     +18.99% |              -53.73% |           0.536 -> 0.253 |            0 -> 0 |
| sharded      |    1000 |                     +13.29% |              -61.94% |           0.388 -> 0.087 |            0 -> 0 |
| thread_local |     100 |                     +62.25% |              -80.43% |           0.377 -> 0.148 |            0 -> 0 |
| thread_local |     500 |                     -53.33% |              +83.54% |           1.142 -> 0.983 |    15960 -> 15980 |
| thread_local |    1000 |    NA (4-core throughput=0) |    NA (4-core p99=0) |           0.000 -> 1.732 |    30000 -> 29980 |

### 4.5 Contention Measurements

Lock wait data from `cmdstat.txt` (global mutex only):

| Dataset                                    | Configuration       | cmdstat_lock_wait_us | Sum(total_time_us) | Estimated contention ratio | GET calls | SET calls |
| ------------------------------------------ | ------------------- | -------------------: | -----------------: | -------------------------: | --------: | --------: |
| final_matrix/20260408_154844               | 4-core global_mutex |                22291 |             364273 |                    6.1193% |     60418 |     59582 |
| final_matrix/20260408_154844               | 8-core global_mutex |                33634 |             424261 |                    7.9277% |     59750 |     60250 |
| metrics_strategy_mandatory/20260408_010639 | global_mutex        |                74304 |             764382 |                    9.7208% |     60040 |     59960 |

Lock-wait correlation entries (factual comparison to matrix cells):

- Final matrix global mutex lock_wait_us: `33634` (8-core) > `22291` (4-core).
- At c=500, throughput overhead vs disabled: `+14.89%` (8-core) vs `+9.13%` (4-core).
- At c=1000, throughput overhead vs disabled: `-21.00%` (8-core) vs `+18.15%` (4-core).

## 5. Observed Phenomena (STRICTLY FACTUAL)

- In the primary matrix, only `thread_local` has non-zero error counts, and only at `500` and `1000` clients.
- `thread_local` records:
  - 4-core: errors `15960` (c=500), `30000` (c=1000)
  - 8-core: errors `15980` (c=500), `29980` (c=1000)
- At 4-core and 1000 clients, `thread_local` aggregate throughput is `0.00` ops/sec.
- At 8-core and 1000 clients, `thread_local` aggregate throughput is `11.26` ops/sec with p99 `72800.00` us.
- For `sharded` at 4-core, p99 CV reaches `1.318` at c=1000.
- For `thread_local`, throughput CV reaches `1.142` (4-core c=500), `0.983` (8-core c=500), and `1.732` (8-core c=1000).
- In final matrix `global_mutex` cmdstat, lock wait is reported and non-zero for both core setups.
- In final matrix `disabled`, `global_mutex`, and `sharded`, total errors are `0` for all listed cells.
- Final matrix `thread_local` cmdstat files are empty (`0` bytes) for both core setups.

## 6. Data Integrity Notes

### 6.1 Run counts and completeness

- Primary matrix (`20260408_154844`) has:
  - machine metadata file
  - 8 strategy/core directories (`2 cores x 4 strategies`)
  - benchmark JSON for all 8 directories
  - `runs_per_config=3` in all primary JSON outputs
- In each successful per-run record, `total_ops` is `10000`.

### 6.2 Known anomalies and missing data

- Incomplete matrix attempt: `results/final_matrix/20260408_154750/`
  - Contains `machine_details.txt`
  - Contains `core_4/disabled/` directory with no files
- Supplementary `results/macos_m2/20260408_005543/summary.csv` contains header only (no rows).
- Supplementary `results/macos_m2/20260408_005543/core_2/thread_local/` has server log only (no benchmark JSON, no cmdstat).
- Supplementary `results/macos_m2/20260408_005543/core_2/sharded/cmdstat.txt` is empty.
- Primary matrix `thread_local` cmdstat files are empty.

### 6.3 Noise considerations

- Memory samples in benchmark JSON are zero on macOS (`rss_bytes=0`, `vsize_bytes=0`) because the benchmark memory reader uses `/proc/self/statm`.
- Server startup logs show AOF replay before each run set; loaded command counts increase across matrix steps (from `909263` to `1290703`).
- Thread-local failures in benchmark outputs are not accompanied by explicit panic/error lines in the corresponding server logs.
- High CV values appear in multiple cells (notably `sharded` and `thread_local`), indicating high run-to-run dispersion in those cells.

### 6.4 Supplementary datasets retained

Mandatory strategy run (`results/metrics_strategy_mandatory/20260408_010639`):

| Strategy     | Clients | Throughput mean | Throughput variance | p99 mean (us) | p99 variance | Errors |
| ------------ | ------: | --------------: | ------------------: | ------------: | -----------: | -----: |
| global_mutex |     100 |       128427.62 |        738483967.62 |       2481.67 |   1370964.33 |      0 |
| global_mutex |     500 |        85309.66 |        921349007.43 |      12181.67 |  93004830.33 |      0 |
| global_mutex |    1000 |        75028.62 |       1019132535.44 |      14862.33 | 127218090.33 |      0 |
| thread_local |     100 |       129197.41 |        422333492.36 |       1992.00 |    567777.00 |      0 |
| thread_local |     500 |        95492.27 |       1607847416.61 |       4622.00 |   1969617.00 |      0 |
| thread_local |    1000 |        94999.85 |        355005531.33 |      17454.67 | 332449576.33 |      0 |

Auxiliary core-2 run (`results/macos_m2/20260408_005543/core_2`):

| Strategy     | Runs | Clients | Throughput mean +- stddev | Throughput CV | p99 mean +- stddev (us) | p99 CV | Errors |
| ------------ | ---: | ------: | ------------------------: | ------------: | ----------------------: | -----: | -----: |
| global_mutex |    5 |     100 |     169753.67 +- 23878.50 |         0.141 |       1315.40 +- 967.02 |  0.735 |      0 |
| global_mutex |    5 |     500 |      157067.66 +- 5800.40 |         0.037 |       3623.80 +- 855.04 |  0.236 |      0 |
| global_mutex |    5 |    1000 |     124016.42 +- 14658.02 |         0.118 |      7048.40 +- 3139.39 |  0.445 |      0 |
| sharded      |    5 |     100 |     154861.94 +- 21999.54 |         0.142 |       1308.40 +- 675.66 |  0.516 |      0 |
| sharded      |    5 |     500 |     138343.53 +- 12743.57 |         0.092 |       3390.60 +- 913.73 |  0.269 |      0 |
| sharded      |    5 |    1000 |     102548.31 +- 29391.57 |         0.287 |    19267.20 +- 29601.22 |  1.536 |   2120 |


---

## Source File: docs/metrics_strategy_mandatory_results_20260408.md

# Metrics Strategy Mandatory Runs (2026-04-08)

## Step 1 (Mandatory): Executed Matrix

- Strategies: `GlobalMutex`, `ThreadLocal`
- Client configurations: `100`, `500`, `1000`
- Workload: `mixed` (50% GET / 50% SET)
- Runs per configuration: `3`
- Raw artifacts:
  - `results/metrics_strategy_mandatory/20260408_010639/global_mutex/benchmark_results.json`
  - `results/metrics_strategy_mandatory/20260408_010639/thread_local/benchmark_results.json`

## Step 2: Collected Metrics

| Strategy    | Clients | Throughput mean (ops/sec) | Throughput variance | p99 mean (us) | p99 variance |
| ----------- | ------: | ------------------------: | ------------------: | ------------: | -----------: |
| GlobalMutex |     100 |                 128427.62 |        738483967.62 |       2481.67 |   1370964.33 |
| GlobalMutex |     500 |                  85309.66 |        921349007.43 |      12181.67 |  93004830.33 |
| GlobalMutex |    1000 |                  75028.62 |       1019132535.44 |      14862.33 | 127218090.33 |
| ThreadLocal |     100 |                 129197.41 |        422333492.36 |       1992.00 |    567777.00 |
| ThreadLocal |     500 |                  95492.27 |       1607847416.61 |       4622.00 |   1969617.00 |
| ThreadLocal |    1000 |                  94999.85 |        355005531.33 |      17454.67 | 332449576.33 |

Variance formula used: `variance = (stddev)^2`.

## Step 3: Results To Send

- At `100` clients:
  - GlobalMutex: throughput `128427.62` ops/sec, p99 `2481.67` us, throughput variance `738483967.62`, p99 variance `1370964.33`
  - ThreadLocal: throughput `129197.41` ops/sec, p99 `1992.00` us, throughput variance `422333492.36`, p99 variance `567777.00`
- At `500` clients:
  - GlobalMutex: throughput `85309.66` ops/sec, p99 `12181.67` us, throughput variance `921349007.43`, p99 variance `93004830.33`
  - ThreadLocal: throughput `95492.27` ops/sec, p99 `4622.00` us, throughput variance `1607847416.61`, p99 variance `1969617.00`
- At `1000` clients:
  - GlobalMutex: throughput `75028.62` ops/sec, p99 `14862.33` us, throughput variance `1019132535.44`, p99 variance `127218090.33`
  - ThreadLocal: throughput `94999.85` ops/sec, p99 `17454.67` us, throughput variance `355005531.33`, p99 variance `332449576.33`

## Step 4 (User-owned)

- Write Results section
- Define final claims
- Structure full paper
