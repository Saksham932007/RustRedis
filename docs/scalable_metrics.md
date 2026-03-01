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
