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
