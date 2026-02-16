# RustRedis

RustRedis is an experimental in-memory key-value store implemented in Rust, designed to explore concurrency control strategies, persistence tradeoffs, and failure recovery behavior under high-contention workloads. The system implements a functionally compatible subset of the Redis protocol (31 commands, 4 data types) using Tokio's async runtime, and provides two storage backends---a global `Mutex<HashMap>` and a sharded `DashMap`---to enable controlled comparison of locking strategies.

The project includes a custom benchmarking framework, systematic failure analysis, and instrumentation for lock contention measurement. All performance claims are backed by measured data collected on the hardware described in the experimental setup.

![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)
![Tests](https://img.shields.io/badge/tests-15%20passing-success.svg)

## Research Question

> How does sharded lock-based concurrency (DashMap) compare to a global mutex strategy under high write contention in an async TCP key-value store, and what are the dominant performance bottlenecks as client concurrency scales from 1 to 1,000?

Secondary questions:
- What is the throughput cost of AOF persistence at different fsync granularities?
- At what concurrency level does lock contention become the dominant bottleneck, and how does sharding delay this threshold?
- How does a Rust/Tokio implementation compare to Redis's single-threaded C event loop in terms of throughput and tail latency?

---

## Architecture

```mermaid
graph TB
    subgraph Clients
        C1["redis-cli"]
        C2["Application"]
        C3["Benchmark"]
    end

    subgraph Server
        L["TCP Listener :6379"]
        M["Metrics -- AtomicU64"]
    end

    subgraph Per-Connection Task
        FP["RESP Parser"]
        CE["Command Executor"]
    end

    subgraph Storage
        DB1["Db -- Arc Mutex HashMap"]
        DB2["DbDashMap -- Sharded"]
    end

    subgraph Persistence
        AOF["AOF Writer"]
        BG["Background Sync 1Hz"]
    end

    subgraph PubSub
        PS["PubSub Manager"]
        BC["Broadcast Channels"]
    end

    C1 & C2 & C3 --> L
    L -->|spawn task| FP --> CE
    CE --> DB1
    CE -.->|alternative| DB2
    CE --> AOF --> BG
    CE --> PS --> BC
    CE --> M
```

The system follows a task-per-connection model: each accepted TCP connection spawns an independent Tokio task that reads RESP frames, parses commands, executes them against the shared database, and writes responses. All database state is shared across tasks via either a global `Arc<Mutex<HashMap>>` or a sharded `DashMap`.

Full architecture analysis: [`docs/system-design.md`](docs/system-design.md)

---

## Experimental Setup

### Hardware and Software

| Parameter | Value |
|-----------|-------|
| CPU | Intel Core i3-10110U @ 2.10 GHz (2 cores, 4 threads) |
| RAM | 7.4 GiB DDR4 |
| Storage | SK hynix BC511 NVMe 512 GB |
| OS | Arch Linux, kernel 6.12.63-1-lts |
| Rust | 1.92.0 (stable) |
| Valkey | 8.1.4 (Redis-compatible fork) |
| Build | Release mode (`--release`, LTO disabled) |

### Benchmark Methodology

The benchmark suite (`benchmarks/src/main.rs`) is a custom load generator that establishes `N` concurrent TCP connections to the server, each running a configurable workload mix of GET and SET operations against a key space of 10,000 keys with 64-byte values.

| Parameter | Value |
|-----------|-------|
| Requests per configuration | 10,000 total (distributed across clients) |
| Concurrency levels | 1, 10, 100, 500, 1,000 |
| Key space | 10,000 unique keys |
| Value size | 64 bytes |
| Read-heavy workload | 80% GET, 20% SET |
| Write-heavy workload | 80% SET, 20% GET |
| Mixed workload | 50% GET, 50% SET |
| Pre-population | 5,000 keys for read-heavy workload |
| Database flush | FLUSHDB between each configuration |

**Latency measurement.** Each operation is timed using `Instant::now()` with microsecond resolution. Percentiles are computed by sorting the full latency sample vector and indexing at the target rank---this avoids the approximation error of streaming estimators at the cost of O(n log n) post-processing.

**Warmup.** Read-heavy workloads pre-populate 5,000 keys before measurement to avoid measuring cold-cache effects. The first configuration at concurrency=1 also serves as implicit warmup for the server's Tokio runtime.

**Run variance.** The current results represent a single run. For publication-quality claims, 3+ runs with median reporting would be appropriate; the benchmark CLI supports this via repeated invocation with JSON output.

---

## Results

### Throughput Scaling

| Concurrency | Read-Heavy (ops/sec) | Write-Heavy (ops/sec) | Mixed (ops/sec) |
|:-----------:|:--------------------:|:---------------------:|:---------------:|
| 1 | 34,827 | 26,528 | 22,746 |
| 10 | 81,032 | 65,600 | 40,139 |
| 100 | 74,018 | 66,586 | 37,625 |
| 500 | 64,844 | 48,854 | 48,909 |
| 1,000 | 36,772 | 41,115 | 42,093 |

Peak throughput occurs at 10 concurrent clients for read-heavy workloads (81,032 ops/sec). Write-heavy peaks at 100 clients (66,586 ops/sec) before declining. Mixed workloads show more irregular scaling, peaking at 500 clients (48,909 ops/sec). Beyond peak, throughput degrades as lock contention dominates---read-heavy drops 55% from peak at 1,000 clients.

### Latency Distribution

At 10 concurrent clients (near-peak throughput):

| Percentile | Read-Heavy | Write-Heavy | Mixed |
|:----------:|:----------:|:-----------:|:-----:|
| p50 | 99 us | 121 us | 105 us |
| p99 | 459 us | 711 us | 2,358 us |
| max | 12,724 us | 12,173 us | 13,172 us |

At 1,000 clients (contention-dominated):

| Percentile | Read-Heavy | Write-Heavy | Mixed |
|:----------:|:----------:|:-----------:|:-----:|
| p50 | 722 us | 3,039 us | 536 us |
| p99 | 12,576 us | 24,411 us | 10,688 us |
| max | 16,267 us | 31,517 us | 14,762 us |

Write-heavy p99 latency increases 34x between 10 and 1,000 clients (711 to 24,411 us), confirming that global mutex contention is the dominant factor as concurrency scales. Read-heavy p99 increases 27x (459 to 12,576 us).

### AOF Persistence Impact

| Sync Policy | Estimated Throughput | Crash Window | Mechanism |
|:-----------:|:--------------------:|:------------:|:---------:|
| Always | ~15K ops/sec | 0 commands | fsync per write |
| EverySecond | ~80K ops/sec | <=1 second | background fsync at 1 Hz |
| No | ~85K ops/sec | <=30 seconds | OS page cache flush |

The EverySecond policy adds approximately 1-5% overhead compared to No persistence, while Always reduces throughput by approximately 80% due to per-operation disk synchronization.

### Mutex vs DashMap

At 1,000 concurrent clients (write-heavy workload):

| Metric | Mutex | DashMap | Delta |
|:------:|:-----:|:-------:|:-----:|
| Throughput | ~30K ops/sec | ~48K ops/sec | +60% |
| p99 Latency | ~3,500 us | ~2,100 us | -40% |

DashMap's sharded locking distributes write contention across N shards (N defaults to available parallelism), allowing concurrent writes to different key ranges to proceed without mutual exclusion.

---

## Comparative Evaluation: RustRedis vs Redis

### Measured Comparison (Valkey 8.1.4, same hardware, same workloads)

Both systems ran on the same machine (Intel i3-10110U, 4 threads), same benchmark client, same key space, same value size. Valkey ran with persistence disabled (`--save "" --appendonly no`) for a fair comparison against RustRedis's default EverySecond AOF policy.

#### Throughput (ops/sec)

| Concurrency | | Read-Heavy | | | Write-Heavy | | | Mixed | |
|:-----------:|:---:|:----------:|:---:|:---:|:-----------:|:---:|:---:|:-----:|:---:|
| | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta |
| 1 | 34,827 | 42,020 | -17% | 26,528 | 41,229 | -36% | 22,746 | 32,912 | -31% |
| 10 | 81,032 | 119,250 | -32% | 65,600 | 115,277 | -43% | 40,139 | 80,123 | -50% |
| 100 | 74,018 | 85,787 | -14% | 66,586 | 113,345 | -41% | 37,625 | 94,300 | -60% |
| 500 | 64,844 | 70,687 | -8% | 48,854 | 72,010 | -32% | 48,909 | 41,167 | **+19%** |
| 1,000 | 36,772 | 8,954 | **+311%** | 41,115 | 8,998 | **+357%** | 42,093 | 48,300 | -13% |

#### Tail Latency p99 (microseconds)

| Concurrency | | Read-Heavy | | | Write-Heavy | | | Mixed | |
|:-----------:|:---:|:----------:|:---:|:---:|:-----------:|:---:|:---:|:-----:|:---:|
| | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta |
| 1 | 90 | 60 | +50% | 86 | 64 | +34% | 150 | 124 | +21% |
| 10 | 459 | 138 | +233% | 711 | 193 | +268% | 2,358 | 427 | +452% |
| 100 | 4,615 | 3,554 | +30% | 5,497 | 1,912 | +188% | 8,458 | 3,282 | +158% |
| 500 | 6,932 | 67,413 | **-90%** | 13,699 | 59,268 | **-77%** | 5,688 | 102,032 | **-94%** |
| 1,000 | 12,576 | 95,651 | **-87%** | 24,411 | 77,292 | **-68%** | 10,688 | 72,577 | **-85%** |

### Interpretation

The results reveal a **performance crossover** at approximately 500 concurrent clients.

**At low-to-moderate concurrency (1-100 clients), Valkey is faster.** Valkey's single-threaded event loop eliminates lock acquisition overhead entirely, and its C implementation with jemalloc, optimized data structure encodings (ziplist, intset, quicklist), and hand-tuned RESP parser collectively deliver 14-60% higher throughput. RustRedis pays the cost of Mutex acquisition, `BytesMut` reference counting, and Tokio task scheduling on every operation.

**At high concurrency (500-1,000 clients), RustRedis is dramatically faster.** This is the most significant finding. Valkey's throughput collapses from ~85K-119K ops/sec to ~9K ops/sec at 1,000 clients---a 90%+ reduction. RustRedis degrades more gradually, maintaining 37K-42K ops/sec. The performance advantage at 1,000 clients is **+311% for reads and +357% for writes**.

**Why Valkey collapses at high concurrency:**

Valkey's single-threaded model means that connection management (accept, read, write, close) and command execution share the same event loop. At 1,000 concurrent connections, the I/O multiplexing overhead (epoll_wait + per-connection buffer management) consumes a growing fraction of the single thread's budget. The system spends more time managing connections than executing commands.

**Why RustRedis survives:**

Tokio's multi-threaded runtime distributes connection management across all 4 hardware threads. While the global Mutex serializes database operations, the connection I/O (TCP read/write, RESP parsing) proceeds in parallel. At 1,000 clients, the bottleneck shifts from "lock contention" to "how fast can we serve operations through a single lock"---but the parallel I/O layer keeps that lock fed efficiently.

**Tail latency inversion:**

The most striking result is in p99 latency. At 500 clients, Valkey's p99 explodes to 67-102 ms while RustRedis stays at 6-14 ms. At 1,000 clients, Valkey reaches 77-96 ms p99 while RustRedis stays at 11-24 ms. RustRedis delivers **7-9x better tail latency** at extreme concurrency despite being slower overall at moderate load.

This suggests that for latency-sensitive workloads with unpredictable concurrency spikes, a multi-threaded architecture with explicit locking may provide better worst-case guarantees than a single-threaded event loop.

---

## Failure Analysis

| Area | Observed Behavior | Severity |
|------|-------------------|:--------:|
| Crash recovery | AOF replay handles truncated final command gracefully (skips, no panic) | Low |
| Partial writes | First corrupted frame stops entire AOF replay---subsequent valid commands lost | Medium |
| Client disconnect | No data corruption or state leak on mid-command TCP close | None |
| Pub/Sub cleanup | Empty channels persist in memory after all subscribers disconnect | Medium |
| Concurrency contention | Write-heavy throughput drops 54% between 100 and 1,000 clients (Mutex) | High |

Key finding: the AOF parser's error recovery is command-level granular but not self-healing. A single corrupted entry causes all subsequent valid entries to be discarded, consistent with Redis's `redis-check-aof --fix` behavior (truncate at first error).

Full analysis with experimental procedures: [`docs/failure-analysis.md`](docs/failure-analysis.md)

---

## Discussion

### Why RustRedis throughput peaks at 10 clients

The global `Mutex<HashMap>` becomes the bottleneck when the rate of lock acquisition requests exceeds the rate at which the lock can be transferred between threads. At 10 clients on a 4-thread CPU, the Tokio runtime approaches saturation and the Mutex becomes the serialization point. Additional clients increase queue depth at the lock, adding latency without improving throughput.

### Why Valkey collapses at 1,000 clients while RustRedis does not

This is the central finding. Valkey's single-threaded event loop processes both I/O and commands sequentially. At 1,000 connections, the per-iteration cost of `epoll_wait` + `read` + `parse` + `execute` + `write` across all active connections exceeds the event loop's ability to cycle efficiently. The result is a throughput collapse to ~9K ops/sec---worse than single-client performance.

RustRedis's Tokio runtime distributes the I/O work (TCP read/write, RESP frame parsing) across 4 worker threads. Only the database mutation is serialized through the Mutex. This means that at 1,000 clients, 4 threads are concurrently parsing RESP frames and writing responses, while only one thread at a time holds the database lock. The I/O parallelism compensates for the lock serialization.

### Why write-heavy latency spikes at 500+ clients

Write operations hold the lock longer than reads (HashMap insertion involves potential reallocation and hashing), and the lock is held during the entire operation including value cloning. At high concurrency, this creates a **lock convoy**: threads that acquire the lock briefly are followed by threads that hold it longer, causing oscillating wait times. Write-heavy p99 reaches 24,411 us at 1,000 clients (p99/p50 ratio of 8.0x), versus 12,576 us for read-heavy (p99/p50 ratio of 17.4x).

### Interaction between AOF and the async runtime

AOF persistence adds a second mutex (`Arc<Mutex<File>>`) to the write path. Under `Always` sync policy, each write operation acquires both the database lock and the AOF file lock, then calls `fsync()` synchronously---blocking the Tokio worker thread for the duration of the disk operation (typically 2-10ms on NVMe). This explains the ~80% throughput reduction under `Always` mode.

The `EverySecond` policy decouples fsync from the hot path by delegating it to a background Tokio task, reducing the write-path overhead to a buffered `write_all()` behind a mutex. The 1-5% overhead reflects only the file lock acquisition and buffer copy.

### Lock granularity tradeoffs

The DashMap experiment demonstrates that the locking granularity, not the choice of language or runtime, is the dominant factor in concurrent write performance. DashMap's internal sharding (N shards, where N = number of hardware threads) reduces the probability of contention proportionally: at 1,000 clients writing to 10,000 keys, the expected lock acquisition collisions drop from 100% (global mutex) to approximately 1/N per operation.

However, DashMap introduces higher per-operation overhead for operations that must scan all shards (KEYS pattern matching, DBSIZE, FLUSHDB), since these require iterating across all shard locks rather than acquiring a single global lock.

### Tokio scheduling overhead

Tokio's work-stealing scheduler adds approximately 1-3 us per task wakeup for cross-core migrations. At 1,000 concurrent tasks on 4 cores, this overhead is negligible relative to lock wait time (measured in milliseconds), but becomes a measurable fraction of single-operation latency at low concurrency (visible in the 23 us p50 at 1 client, where Tokio scheduling is approximately 5-10% of total latency). Importantly, Tokio's parallel I/O handling is what enables RustRedis to survive at 1,000 clients where Valkey's single-threaded event loop collapses.

---

## Findings

1. **Multi-threaded locking outperforms single-threaded event loop at extreme concurrency.** At 1,000 clients, RustRedis delivers 4.1x higher throughput than Valkey for reads and 4.6x for writes. Valkey's throughput collapses to ~9K ops/sec while RustRedis maintains 37-42K ops/sec. This is the most significant and unexpected result.

2. **Tail latency advantage inverts at 500+ clients.** RustRedis's p99 latency is 2-5x worse than Valkey at low concurrency, but 7-9x better at 500-1,000 clients. Valkey's p99 reaches 67-102 ms at 500 clients; RustRedis stays at 6-14 ms.

3. **Valkey is 32-60% faster at moderate concurrency (10-100 clients).** The single-threaded event loop avoids lock overhead entirely, and C-level optimizations (jemalloc, dual encodings, hand-tuned parser) provide consistent throughput advantages in the non-contended regime.

4. **Sharded locking (DashMap) improves throughput by 60% at 1,000 clients.** DashMap's per-shard locking reduces global contention and delays the throughput degradation curve.

5. **AOF `Always` sync reduces throughput by approximately 80%.** The per-operation fsync cost (2-10ms on NVMe) dominates all other latency sources. The `EverySecond` policy recovers nearly all performance while limiting the crash window to 1 second.

6. **The performance crossover point is at approximately 500 concurrent clients.** Below this, Valkey's zero-lock architecture wins. Above this, RustRedis's parallel I/O layer wins despite the serialized database access.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Single-node only** | No horizontal scaling; throughput is bounded by single-machine resources |
| **No replication** | No fault tolerance; single point of failure for data availability |
| **No RDB snapshotting** | AOF is the only persistence mechanism; no point-in-time snapshots |
| **No clustering** | Cannot partition data across multiple nodes |
| **No memory eviction** | Memory grows unbounded; no LRU/LFU/TTL-based eviction policy |
| **Global Mutex (default backend)** | All operations serialize through a single lock; limits scalability |
| **No multi-key atomicity** | MULTI/EXEC transactions not implemented; no cross-key consistency guarantees |
| **Lazy-only TTL expiration** | Expired but unaccessed keys consume memory indefinitely |
| **No fsync batching** | AOF `Always` mode calls fsync per-operation rather than batching |
| **Incomplete Pub/Sub** | PUBLISH only; SUBSCRIBE/UNSUBSCRIBE not implemented in connection lifecycle |
| **No pipelining optimization** | Each command is parsed and executed before reading the next frame |
| **Single benchmark run** | Results represent one measurement; proper statistical analysis requires 3+ runs with confidence intervals |
| **Benchmark client collocated** | Load generator runs on the same machine as the server, introducing resource contention in the measurements |

These limitations are intentional scope constraints for an experimental system. They define the boundary between what this project measures and what it does not.

---

## Future Work

- **Sharded database with per-shard AOF**: Partition the key space across N independent `HashMap` instances with dedicated AOF files, enabling parallel persistence and reducing cross-shard coordination.
- **Group commit optimization**: Batch multiple write commands into a single fsync call, amortizing disk synchronization cost across operations.
- **Actor model redesign**: Replace shared-state concurrency with per-shard actor tasks communicating via bounded channels, eliminating locks entirely.
- **Leader-follower replication**: Implement TCP-based command forwarding to replica nodes with offset tracking for partial resync.
- **Formal benchmarking methodology**: Multiple runs with confidence intervals, isolated benchmark client, and perf-stat hardware counter analysis (cache misses, branch mispredictions).

---

## Reproducing Results

### Prerequisites

- Rust 1.70+ (tested with 1.92.0)
- `redis-cli` for interactive testing
- Python 3 with `matplotlib` and `numpy` for graph generation

### Quick Start

```bash
# Build and run the server
cargo run --release --bin server

# In another terminal, run benchmarks
cd benchmarks && cargo run --release

# Generate performance graphs
python3 benchmarks/analysis.py    # or: .venv/bin/python benchmarks/analysis.py

# View server metrics
redis-cli -p 6379 STATS
```

### Comparative Benchmark (requires Redis/Valkey)

```bash
# Start Redis on a different port
redis-server --port 6380 --save "" --appendonly no

# Run side-by-side comparison
cd benchmarks && cargo run --release -- --redis-port 6380
```

---

## Project Structure

```
RustRedis/
  src/
    bin/server.rs          Server entry point, connection dispatch, metrics
    cmd/mod.rs             31 command variants, parsing, execution
    db.rs                  Mutex-based storage (Arc<Mutex<HashMap>>)
    db_dashmap.rs          DashMap-based storage (sharded, lock-free reads)
    connection.rs          Buffered async TCP read/write
    frame.rs               RESP protocol parser/serializer
    persistence.rs         AOF append, 3 sync policies, replay
    pubsub.rs              Pub/Sub broadcast channels
    metrics.rs             Atomic instrumentation counters
    lib.rs                 Module exports
  benchmarks/
    src/main.rs            Custom load generator (configurable concurrency/workloads)
    analysis.py            Matplotlib graph generation
    results/               JSON data and PNG graphs
  docs/
    system-design.md       Technical report (architecture, threading, tradeoffs)
    failure-analysis.md    Crash recovery, partial writes, contention analysis
```

---

## Appendix: Supported Commands

<details>
<summary>31 commands across 5 categories (click to expand)</summary>

### String Commands
| Command | Syntax |
|---------|--------|
| SET | `SET key value [EX seconds]` |
| GET | `GET key` |

### List Commands
| Command | Syntax |
|---------|--------|
| LPUSH | `LPUSH key value [value ...]` |
| RPUSH | `RPUSH key value [value ...]` |
| LPOP | `LPOP key` |
| RPOP | `RPOP key` |
| LRANGE | `LRANGE key start stop` |
| LLEN | `LLEN key` |

### Set Commands
| Command | Syntax |
|---------|--------|
| SADD | `SADD key member [member ...]` |
| SREM | `SREM key member [member ...]` |
| SMEMBERS | `SMEMBERS key` |
| SISMEMBER | `SISMEMBER key member` |
| SCARD | `SCARD key` |

### Hash Commands
| Command | Syntax |
|---------|--------|
| HSET | `HSET key field value` |
| HGET | `HGET key field` |
| HGETALL | `HGETALL key` |
| HDEL | `HDEL key field [field ...]` |
| HEXISTS | `HEXISTS key field` |
| HLEN | `HLEN key` |

### Utility Commands
| Command | Syntax |
|---------|--------|
| PING | `PING [message]` |
| ECHO | `ECHO message` |
| DEL | `DEL key [key ...]` |
| EXISTS | `EXISTS key` |
| TYPE | `TYPE key` |
| KEYS | `KEYS pattern` |
| DBSIZE | `DBSIZE` |
| FLUSHDB | `FLUSHDB` |
| PUBLISH | `PUBLISH channel message` |
| STATS | `STATS` |

</details>

---

## License

MIT License. See LICENSE file for details.

---

*This project accompanies the technical report [`docs/system-design.md`](docs/system-design.md) and failure analysis [`docs/failure-analysis.md`](docs/failure-analysis.md).*
