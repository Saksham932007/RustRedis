# Scalable Concurrent Metrics Implementation Prompt

**Copy and paste the following prompt to an LLM or AI coding agent to implement the Scalable Metrics feature.**

***

## Context & Role
You are an elite Rust systems engineer and distributed systems researcher. You are working on **RustRedis**, an experimental in-memory key-value store implemented in Rust using Tokio. The system aims to explore concurrency control strategies under high-contention workloads.

Currently, the server tracks global metrics (like `total_commands`, `active_connections`, `total_command_duration_us`) using simple `AtomicU64` counters in `src/metrics.rs` with `Ordering::Relaxed`. While this approach is fast for global totals, it does not provide granular insights into individual command performance.

## Objective
Your task is to analyze the existing codebase and implement a **Scalable Concurrent Metrics Collection system** under high contention, heavily inspired by PostgreSQL's `pg_stat_statements`. Because adding fine-grained telemetry to the hot path can severely degrade performance, you must implement and compare three different concurrency models for metrics collection.

## Execution Steps

### 1. Codebase Analysis
- Read and understand `src/metrics.rs`, `src/cmd/mod.rs` (where the execution happens), and `src/bin/server.rs`.
- Identify the hot path where commands are processed and responses are written. This is where the new telemetry collection hooks will be inserted.
- Understand the existing Tokio async architecture and task-per-connection model.

### 2. Feature Implementation: The Metrics Collector
Implement a `CommandStats` tracker that records, for each command type (e.g., `GET`, `SET`, `HGET`):
- `calls`: Total number of times the command was invoked.
- `total_time_us`: Cumulative execution time in microseconds.
- `min_time_us`: Minimum execution time observed.
- `max_time_us`: Maximum execution time observed.

You must implement three different backend strategies for storing and updating these statistics to evaluate their contention characteristics. They should be toggleable (e.g. via an Enum or configuration flag at startup):

#### A. Global Lock Baseline (`MetricsStrategy::GlobalMutex`)
- Implement the collector using a single global `std::sync::Mutex<HashMap<&'static str, CommandStat>>`.
- Every command execution requires acquiring this global lock to update its statistics.
- Instrument the time spent waiting for this lock and expose it.

#### B. Sharded Metrics System (`MetricsStrategy::Sharded`)
- Replace the single Mutex with an array of Mutexes (e.g., an array of 16-64 `Mutex<CommandStat>`) partitioned by command type or command hash. Alternatively, use `DashMap`.
- This reduces contention by allowing parallel updates for different commands.

#### C. Thread-local / Reduced-lock System (`MetricsStrategy::ThreadLocalBatched`)
- Implement a lock-free or reduced-lock approach using Thread-Local Storage (TLS) or Tokio task-local state.
- Each worker thread maintains its own local counters.
- Periodically (e.g., every 100ms or every 1000 commands), the local batch is aggregated and flushed to a global snapshot, keeping the hot path completely free of heavy atomic operations or locking barriers.

### 3. Exposing the Data
- Add a new RESP command: `CMDSTAT` (or extend the existing `STATS` command) to emit these granular per-command telemetry metrics to the client.

### 4. Benchmark Suite & Contention Analysis
- Update `benchmarks/src/main.rs`.
- Add a methodology to test the server with the different metrics collection strategies.
- Add dedicated output describing the throughput (ops/sec) and p99 latency degradation compared to when telemetry is completely disabled.

### 5. Update the README and Documentation
- Provide a detailed update to `README.md`. 
- Incorporate a new section titled **"Scalable Concurrent Metrics Collection"**.
- Document the three strategies.
- Include a **Contention Analysis & Performance Comparison** section detailing the hypothetical (or empirically tested if you run it) performance overhead of `GlobalLock` vs `Sharded` vs `ThreadLocalBatched` on a multi-core machine.
- Explain how thread-local batching solves the lock convoy effect on the telemetry hot path.

### Formatting & Output Requirements
- Do not remove existing logic. Safely integrate the hooks into the command execution router.
- Ensure your code cleanly compiles with strict Rust standard practices. 
- Ensure `Cargo.toml` dependencies are updated if `DashMap` or `thread_local` components are introduced.
- Modify `README.md` in place, adding the new findings clearly directly under the "Results" and "Architecture" sections.
