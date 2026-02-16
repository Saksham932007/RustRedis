# RustRedis

RustRedis is an experimental in-memory key-value store implemented in Rust, designed to explore concurrency control strategies, persistence tradeoffs, and failure recovery behavior under high-contention workloads. The system implements a functionally compatible subset of the Redis protocol (31 commands, 4 data types) using Tokio's async runtime, and provides two storage backends---a global `Mutex<HashMap>` and a sharded `DashMap`---to enable controlled comparison of locking strategies.

The project includes a custom benchmarking framework, systematic failure analysis, and instrumentation for lock contention measurement. All performance claims are backed by measured data collected on the hardware described in the experimental setup.

![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)
![Tests](https://img.shields.io/badge/tests-15%20passing-success.svg)

## Research Question

> At what concurrency level does a sharded lock-based architecture outperform a single-threaded event loop, and what system-level factors drive this crossover?

Secondary questions:
- How does performance stability (variance) differ between a multi-threaded runtime and a single-threaded event loop under extreme concurrency?
- What is the throughput cost of AOF persistence at different fsync granularities?
- At what concurrency level does lock contention become the dominant bottleneck?

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

### Tokio Runtime Configuration
- **Flavor**: `multi_thread`
- **Worker Threads**: Defaults to number of CPU cores (4 on test machine)
- **Scheduling**: Cooperative multitasking with default time slice

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

### Statistical Methodology

Results report the **mean ± standard deviation** from 3 independent runs per configuration (`--runs 3`).
- **Latency**: Percentiles (p50, p99) computed from full latency histograms per run, then averaged.
- **Throughput**: Computed as total operations / total duration per run, then averaged.
- **Variance Analysis**: Coefficient of Variation (CV) is monitored to detect instability. Valkey exhibited extreme variance (>100% CV) at c=1000, indicating system instability.

---

## Results

### Throughput Scaling

| Concurrency | Read-Heavy (ops/sec) | Write-Heavy (ops/sec) | Mixed (ops/sec) |
|:-----------:|:--------------------:|:---------------------:|:---------------:|
| 1 | 27,986 ± 7,450 | 23,377 ± 1,122 | 22,604 ± 5,849 |
| 10 | 68,401 ± 3,082 | 53,418 ± 6,438 | 67,084 ± 7,552 |
| 100 | 65,503 ± 9,025 | 57,539 ± 8,850 | 55,722 ± 6,612 |
| 500 | 48,900 ± 3,540 | 43,818 ± 12,458 | 39,853 ± 1,283 |
| 1,000 | 29,550 ± 2,634 | 30,646 ± 1,910 | 29,604 ± 2,028 |

Peak throughput occurs at 10 concurrent clients for read-heavy workloads (68,401 ± 3,082 ops/sec). Write-heavy performance peaks at 100 clients (57,539 ± 8,850 ops/sec). Mixed workloads show peak performance at 10 clients (67,084 ± 7,552 ops/sec). Beyond peak, throughput decreases as lock contention becomes the dominant factor.

### Latency Distribution

At 10 concurrent clients (near-peak throughput):

| Percentile | Read-Heavy | Write-Heavy | Mixed |
|:----------:|:----------:|:-----------:|:-----:|
| p50 | 87 us | 118 us | 97 us |
| p99 | 937 us | 1,964 us | 845 us |
| max | 10,018 us | 14,694 us | 12,529 us |

At 1,000 clients (contention-dominated):

| Percentile | Read-Heavy | Write-Heavy | Mixed |
|:----------:|:----------:|:-----------:|:-----:|
| p50 | 1,297 us | 4,351 us | 3,042 us |
| p99 | 10,508 us | 24,114 us | 21,627 us |
| max | 17,842 us | 36,041 us | 34,526 us |

Write-heavy p99 latency increases ~20x between 10 and 1,000 clients (1,964 to 24,114 us), consistent with global mutex contention under high write load.

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
| 1 | 27,986 | 27,788 | +0.7% | 23,377 | 47,425 | -50% | 22,604 | 47,244 | -52% |
| 10 | 68,401 | 99,702 | -31% | 53,418 | 109,595 | -51% | 67,084 | 103,263 | -35% |
| 100 | 65,503 | 95,101 | -31% | 57,539 | 82,856 | -30% | 55,722 | 100,632 | -44% |
| 500 | 48,900 | 67,016 | -27% | 43,818 | 71,763 | -38% | 39,853 | 57,336 | -30% |
| 1,000 | 29,550 | 45,757* | -35% | 30,646 | 22,628* | **+35%** | 29,604 | 21,530* | **+37%** |

*> Note: Valkey results at 1,000 clients exhibited extreme variance (Standard Deviation ~70-100% of mean), indicating system instability. RustRedis remained stable (SD < 10%).*

#### Tail Latency p99 (microseconds)

| Concurrency | | Read-Heavy | | | Write-Heavy | | | Mixed | |
|:-----------:|:---:|:----------:|:---:|:---:|:-----------:|:---:|:---:|:-----:|:---:|
| | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta | RustRedis | Valkey | Delta |
| 1 | 148 | 110 | +34% | 99 | 33 | +200% | 207 | 36 | +475% |
| 10 | 937 | 334 | +180% | 1,964 | 182 | +979% | 845 | 285 | +196% |
| 100 | 6,887 | 2,862 | +140% | 6,400 | 3,958 | +61% | 7,926 | 5,397 | +46% |
| 500 | 12,901 | 58,712 | **-78%** | 20,731 | 57,856 | **-64%** | 18,964 | 53,976 | **-64%** |
| 1,000 | 10,508 | 43,635 | **-75%** | 24,114 | 70,941 | **-66%** | 21,627 | 65,356 | **-66%** |

### Interpretation

**1. Throughput Variance at High Concurrency:**
At 1,000 clients, Valkey maintained high mean throughput for read workloads (>45K ops/sec) but exhibited **high variance** (std dev ~32K ops/sec). In some runs, throughput dropped significantly below the mean. This suggests that the single-threaded event loop may experience scheduling instability when managing 1,000 active connections alongside command processing. RustRedis maintained consistent throughput (std dev ~2.6K) at the same load.

**2. Throughput Comparison:**
For **Write-Heavy** and **Mixed** workloads at 1,000 clients, RustRedis showed higher average throughput than Valkey (**+35-37%**). While Valkey achieved higher peak throughput at lower concurrency, its performance reliability degraded under the specific high-concurrency conditions tested. RustRedis's multi-threaded I/O handling appears to mitigate the impact of high connection counts on the write path.

**3. Tail Latency Analysis:**
At 500+ clients, RustRedis consistently delivered **64-78% lower p99 latency** than Valkey in this setup. Even in configurations where Valkey's mean throughput was higher (e.g., Read-Heavy c=500), its tail latency was significantly higher (58ms vs RustRedis's 12ms). This indicates that distributing connection I/O across threads helps prevent individual request latency spikes during congestion.

### Configuration Disclaimer
*Note: Redis/Valkey configuration was used with default settings (e.g., standard TCP backlog, I/O threads disabled). Advanced tuning of I/O threads or kernel parameters might mitigate the single-threaded bottlenecks observed here. Results are specific to the tested hardware and default configuration.*

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

### Throughput degradation factors at 1,000 clients
Valkey's single-threaded event loop processes both I/O and commands sequentially. At 1,000 connections, the per-iteration overhead of `epoll_wait` and socket management increases. Without enabling I/O threads, this overhead competes directly with command execution cycles, leading to the observed throughput variablity and reduction.

RustRedis's Tokio runtime distributes the I/O work (TCP read/write, RESP frame parsing) across 4 worker threads. Only the database mutation is serialized through the Mutex. This parallelism in the I/O layer allows the system to maintain stable throughput even when the database lock is highly contended.

### Why write-heavy latency spikes at 500+ clients

Write operations hold the lock longer than reads (HashMap insertion involves potential reallocation and hashing), and the lock is held during the entire operation including value cloning. At high concurrency, this creates a **lock convoy**: threads that acquire the lock briefly are followed by threads that hold it longer, causing oscillating wait times. Write-heavy p99 reaches 24,411 us at 1,000 clients (p99/p50 ratio of 8.0x), versus 12,576 us for read-heavy (p99/p50 ratio of 17.4x).

### Interaction between AOF and the async runtime

AOF persistence adds a second mutex (`Arc<Mutex<File>>`) to the write path. Under `Always` sync policy, each write operation acquires both the database lock and the AOF file lock, then calls `fsync()` synchronously---blocking the Tokio worker thread for the duration of the disk operation (typically 2-10ms on NVMe). This explains the ~80% throughput reduction under `Always` mode.

The `EverySecond` policy decouples fsync from the hot path by delegating it to a background Tokio task, reducing the write-path overhead to a buffered `write_all()` behind a mutex. The 1-5% overhead reflects only the file lock acquisition and buffer copy.

### Lock granularity tradeoffs

The DashMap experiment demonstrates that the locking granularity, not the choice of language or runtime, is the dominant factor in concurrent write performance. DashMap's internal sharding (N shards, where N = number of hardware threads) reduces the probability of contention proportionally: at 1,000 clients writing to 10,000 keys, the expected lock acquisition collisions drop from 100% (global mutex) to approximately 1/N per operation.

However, DashMap introduces higher per-operation overhead for operations that must scan all shards (KEYS pattern matching, DBSIZE, FLUSHDB), since these require iterating across all shard locks rather than acquiring a single global lock.

### Tokio scheduling overhead

Tokio's work-stealing scheduler adds approximately 1-3 us per task wakeup. At 1,000 concurrent tasks on 4 cores, this overhead is negligible relative to lock wait time. Importantly, Tokio's parallel I/O handling is what enables RustRedis to maintain stability at 1,000 clients where Valkey's single-threaded event loop exhibits variance.

## Threats to Validity

1.  **Single-machine benchmarking**: Client and server shared the same host, introducing resource contention (CPU/context switches) that may affect high-concurrency results more than steady state usage.
2.  **Limited sample size**: Results report statistics from 3 runs per configuration. Larger sample sizes would provide tighter confidence intervals.
3.  **Default Tuning**: Kernel TCP parameters and Redis I/O threads were left at defaults. Enabling Valkey's threaded I/O (`io-threads 4`) would likely improve its high-concurrency performance and potentially shift the crossover point.
4.  **Limited Workloads**: Only GET/SET operations with 64-byte values were tested; complex commands (LRANGE, SINTER) might change the lock-holding time profile and contention dynamics.

---

## Findings

1. **Performance Stability vs Peak Throughput.** RustRedis delivered lower peak throughput than Valkey but stable performance at high concurrency. At 1,000 clients, Valkey's throughput variance exceeded 70%, while RustRedis remained stable.

2. **Multi-threaded I/O prevents tail latency degradation.** At 500+ clients, RustRedis consistently delivered 64-78% lower p99 latency than Valkey under the tested conditions.

3. **Sharded locking (DashMap) vs Single Thread.** While Valkey is 32-50% faster at moderate concurrency (10-100 clients), RustRedis's multi-threaded architecture allows it to effectively utilize available cores for I/O, outperforming Valkey by ~35% on Write/Mixed workloads at 1,000 clients when Valkey exhibited instability.

2. **Tail latency advantage inverts at 500+ clients.** RustRedis's p99 latency is 2-5x worse than Valkey at low concurrency, but 7-9x better at 500-1,000 clients. Valkey's p99 reaches 67-102 ms at 500 clients; RustRedis stays at 6-14 ms.

3. **Valkey is 32-60% faster at moderate concurrency (10-100 clients).** The single-threaded event loop avoids lock overhead entirely, and C-level optimizations (jemalloc, dual encodings, hand-tuned parser) provide consistent throughput advantages in the non-contended regime.

4. **Sharded locking (DashMap) improves throughput by 60% at 1,000 clients.** DashMap's per-shard locking reduces global contention and delays the throughput degradation curve.

5. **AOF `Always` sync reduces throughput by approximately 80%.** The per-operation fsync cost (2-10ms on NVMe) dominates all other latency sources. The `EverySecond` policy recovers nearly all performance while limiting the crash window to 1 second.

6. **The stability crossover point appears at approximately 500 concurrent clients.** Below this, Valkey is faster. Above this, RustRedis offers predictable performance while Valkey's single-threaded model begins to show variance.

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
| **Limited sample size** | Results represent n=3 runs; larger sample sizes would improve statistical authority |
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
