# RustRedis

[![CI](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml/badge.svg)](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.23295261.svg)](https://doi.org/10.5281/zenodo.23295261)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

RustRedis is a high-performance, Redis-compatible in-memory key-value server written in Rust (~3,500 LOC). It features a multi-threaded asynchronous architecture powered by Tokio, pluggable keyspace backends, append-only file (AOF) persistence, and an extensible telemetry subsystem.

This repository serves two distinct audiences. Choose the path that matches your interest:

---

### Choose Your Path

* **[Path A: Build and Use the Server](#path-a-build-and-use-the-server)**  
  *For backend engineers and systems developers.*  
  Learn about the RESP server implementation, multi-threaded Tokio architecture, AOF persistence engine, supported Redis commands, telemetry strategies, and instructions to run and benchmark the server.

* **[Path B: Understand the Research](#path-b-understand-the-research)**  
  *For researchers, performance engineers, and observability architects.*  
  Explore the empirical study (*"Cardinality Before Contention"*), the factorial experimental design, key research findings, statistical methodology (RMIT), paper manuscript, and reproducibility suite. All exhaustive empirical data tables are preserved in **[docs/research.md](docs/research.md)**.

---

# Path A: Build and Use the Server

## Quick Start

### 1. Prerequisites & Compilation

Ensure you have a recent Rust toolchain installed (edition 2021, Rust 1.75+ recommended):

```bash
# Build the release binary
cargo build --release --bin server
```

### 2. Running the Server

Start the server on the default address (`127.0.0.1:6379`):

```bash
cargo run --release --bin server
```

The server outputs structured logs via `tracing`:
```text
INFO RustRedis server listening on 127.0.0.1:6379
INFO Keyspace store: mutex
INFO Command metrics initialized (strategy: sharded_2key)
INFO AOF persistence enabled with EverySecond sync policy (path: appendonly.aof)
INFO Press CTRL+C to shutdown gracefully
```

### 3. Interacting with the Server

You can connect to RustRedis using standard Redis clients such as `redis-cli`, Python (`redis-py`), Go (`go-redis`), or raw TCP (`nc`, `telnet`):

```bash
# Basic connection check
redis-cli -p 6379 ping
# PONG

# String operations with expiration
redis-cli -p 6379 set session:token "abc123xyz" EX 300
# OK
redis-cli -p 6379 get session:token
# "abc123xyz"

# List operations
redis-cli -p 6379 lpush tasks "task_1" "task_2"
# (integer) 2
redis-cli -p 6379 lrange tasks 0 -1
# 1) "task_2"
# 2) "task_1"

# Telemetry inspection
redis-cli -p 6379 stats
redis-cli -p 6379 cmdstat
```

### 4. Configuration

RustRedis is configured via environment variables:

| Variable | Default | Description |
|---|---|---|
| `RUSTREDIS_BIND` | `127.0.0.1:6379` | TCP socket address and port to bind to. |
| `RUSTREDIS_STORE` | `mutex` | Keyspace storage engine: `mutex` (single global lock) or `dashmap` (sharded concurrent map). |
| `RUSTREDIS_METRICS_STRATEGY` | `sharded_2key` | Metrics telemetry collector (one of 8 strategies, see below). |
| `RUSTREDIS_METRICS_CARDINALITY` | `2` | Active key cardinality for the `sharded_bucketed` collector. |
| `RUSTREDIS_DISABLE_AOF` | `false` | Set to `1`, `true`, or `yes` to disable write persistence. |
| `RUSTREDIS_AOF_PATH` | `appendonly.aof` | File path for the Append-Only File. |

Example: Run a high-concurrency instance with sharded keyspace storage and thread-local metrics:
```bash
RUSTREDIS_STORE=dashmap RUSTREDIS_METRICS_STRATEGY=thread_local cargo run --release --bin server
```

---

## Server Architecture & Internals

```
                       ┌──────────────────────────────────────────────┐
                       │               Client Connections             │
                       └──────────────────────┬───────────────────────┘
                                              │ (TCP)
                                              ▼
                       ┌──────────────────────────────────────────────┐
                       │           Tokio Multi-Threaded Loop          │
                       │            (One Task Per Client)             │
                       └──────────────────────┬───────────────────────┘
                                              │
                                              ▼
                       ┌──────────────────────────────────────────────┐
                       │            RESP Protocol Parser              │
                       │     (Frame extraction, bounds checking)      │
                       └──────────────────────┬───────────────────────┘
                                              │
                       ┌──────────────────────┴───────────────────────┐
                       │                                              │
                       ▼                                              ▼
    ┌─────────────────────────────────────┐        ┌─────────────────────────────────────┐
    │       Command Dispatcher            │        │           AOF Persistence           │
    │   (GET, SET, LPUSH, SADD, etc.)     │        │  (Write-ahead logging, fsync loop)  │
    └──────────────────┬──────────────────┘        └─────────────────────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        ▼                             ▼
┌──────────────────┐        ┌───────────────────────┐
│  Keyspace Store  │        │   Telemetry Collector │
│ (Mutex / DashMap)│        │   (1 of 8 Strategies) │
└──────────────────┘        └───────────────────────┘
```

### 1. RESP Protocol Parser & Framing
The server implements the Redis Serialization Protocol (RESP2) in [src/frame.rs](src/frame.rs) and [src/connection.rs](src/connection.rs):
* **Supported Frame Types**: Simple Strings (`+`), Errors (`-`), Integers (`:`), Bulk Strings (`$`), Arrays (`*`), and Nulls.
* **Hardened Framing**: Zero-copy buffered I/O with `bytes::BytesMut`. Validates integer boundaries, rejects recursive nesting attacks, and handles partial frames cleanly without unbounded buffer allocation.
* **Streaming Network I/O**: Connections read and write asynchronously over `tokio::net::TcpStream` using a buffered flush model to optimize throughput under pipelined workloads.

### 2. Tokio Asynchronous Multi-threaded Runtime
Implemented in [src/bin/server.rs](src/bin/server.rs):
* **Task Isolation**: When a TCP connection is accepted, it is spawned onto the Tokio multi-threaded runtime (`#[tokio::main(flavor = "multi_thread")]`).
* **Decoupled Concurrency**: The server handles thousands of concurrent client connections without spawning an OS thread per connection. The Tokio runtime dispatches async tasks across a worker thread pool sized to host vCPUs.
* **Worker Thread Determinism**: The number of OS threads concurrently entering the database or telemetry hot path equals the host's physical/virtual core count, independent of the number of active network sockets.

### 3. Pluggable Storage Backends (`Store` Trait)
Database backends implement the `Store` trait in [src/store.rs](src/store.rs):
* **`Db` ([src/db.rs](src/db.rs))**: A single global `std::sync::Mutex` protecting a `HashMap<String, Entry>`. This provides standard single-threaded Redis serialization semantics and serves as the baseline reference.
* **`DbDashMap` ([src/db_dashmap.rs](src/db_dashmap.rs))**: A concurrent, 64-way sharded hash map (`dashmap::DashMap`) for concurrent read/write scaling.
* **Expiration Engine**: Passive/lazy expiration on key access across all data types, checking millisecond timestamps before returning values.

### 4. AOF (Append-Only File) Persistence Engine
Implemented in [src/persistence.rs](src/persistence.rs):
* **Write-Ahead Logging**: Mutating commands (`SET`, `DEL`, `LPUSH`, `HSET`, etc.) are serialized into standard RESP array frames and written to disk.
* **Sync Policies**:
  * `Always`: Synchronously calls `fsync()` after every write command (maximum durability, lowest throughput).
  * `EverySecond` (default): Appends writes to the OS buffer cache and flushes to disk once per second via a dedicated background Tokio task.
  * `No`: Delegates sync decisions entirely to the OS buffer management.
* **Crash Recovery Replay**: On server startup, `Aof::load` scans and validates the AOF file, replaying commands sequentially through `Command::replay` to reconstruct keyspace state before accepting client traffic.

### 5. Pub/Sub Subsystem
Implemented in [src/pubsub.rs](src/pubsub.rs):
* Centralized broadcast hub using `tokio::sync::broadcast` channels with bounded capacity (1,024 messages per channel).
* Independent channel lifecycle management with automatic cleanup of inactive channels when subscriber counts drop to zero.

---

## Supported Redis Commands

Implemented in [src/cmd/mod.rs](src/cmd/mod.rs):

| Category | Commands | Description |
|---|---|---|
| **Connection & Server** | `PING [msg]` | Tests server connectivity; echoes optional message. |
| | `ECHO msg` | Returns the input message. |
| | `STATS` | Returns process metrics, active connections, total ops, and AOF timing. |
| | `CMDSTAT` | Returns real-time latency and call counts from the active metrics collector. |
| **Strings & Keyspace** | `GET key` | Retrieves the string value of a key. |
| | `SET key value [EX sec]` | Sets key to value with optional TTL in seconds. |
| | `DEL key [key...]` | Deletes one or more keys. |
| | `EXISTS key` | Returns 1 if key exists, 0 otherwise. |
| | `TYPE key` | Returns data type (`string`, `list`, `set`, `hash`, or `none`). |
| | `DBSIZE` | Returns total keys stored in the active database. |
| | `FLUSHDB` | Removes all keys from the database. |
| | `KEYS pattern` | Returns keys matching glob-style pattern (e.g., `user:*`). |
| **Lists** | `LPUSH key val [val...]` | Prepends one or more elements to a list. |
| | `RPUSH key val [val...]` | Appends one or more elements to a list. |
| | `LPOP key` | Removes and returns the first element of a list. |
| | `RPOP key` | Removes and returns the last element of a list. |
| | `LRANGE key start stop` | Returns range of elements (supports negative indices). |
| | `LLEN key` | Returns list length. |
| **Sets** | `SADD key mem [mem...]` | Adds unique members to a set. |
| | `SREM key mem [mem...]` | Removes members from a set. |
| | `SMEMBERS key` | Returns all members of a set. |
| | `SISMEMBER key mem` | Tests member presence in a set. |
| | `SCARD key` | Returns set cardinality. |
| **Hashes** | `HSET key field val` | Sets field in a hash. |
| | `HGET key field` | Retrieves field value from a hash. |
| | `HGETALL key` | Returns all field-value pairs in a hash. |
| | `HDEL key field [field...]` | Deletes one or more fields from a hash. |
| | `HEXISTS key field` | Tests if a field exists in a hash. |
| | `HLEN key` | Returns count of fields in a hash. |
| **Pub/Sub** | `PUBLISH channel msg` | Posts a message to a channel; returns receiver count. |

---

## Telemetry Collectors: The 8 Implementation Strategies

A central feature of RustRedis is its swappable telemetry collectors in [src/command_metrics.rs](src/command_metrics.rs).

### ⚠️ Important: 6 Main Experiment Strategies vs. 8 Implementation Strategies

To understand the codebase, developers and researchers should note the distinction between the strategy sets:
1. **The 6 Main Experiment Strategies**: The primary empirical study (*"Cardinality Before Contention"*) evaluated **six collectors** configured as a clean $2^2$ factorial design along 4 orthogonal axes.
2. **The 8 Final Implementation Strategies**: During the study, two confounding variables were identified (isolating map size from key string hashing, and isolating histogram tracking from string allocation). To untangle these, **two additional strategies (`sharded_bucketed` and `thread_local_owned`) were implemented** for follow-up axis separation.

All **eight strategies** are fully available in the production server binary:

| Strategy Flag (`RUSTREDIS_METRICS_STRATEGY`) | Scope | Synchronization | Cardinality | Aggregation | Payload Recorded |
|---|---|---|---:|---|---|
| `disabled` | Main & Final | None | 0 | None | None (Bypassed) |
| `global_mutex` | Main & Final | Single `Mutex<HashMap>` | 2 (`GET`/`SET`) | Eager, Shared | 64-bit Counter |
| `sharded_2key` | Main & Final | `DashMap` (64 shards) | 2 (`GET`/`SET`) | Eager, Shared | 64-bit Counter |
| `sharded_n` | Main & Final | `DashMap` (64 shards) | ~10,000 (Data keys) | Eager, Shared | 64-bit Counter |
| `thread_local` | Main & Final | Thread-Local Map | 2 (`GET`/`SET`) | Deferred (100ms flush) | 64-bit Counter |
| `hdr_histogram` | Main & Final | Thread-Local Map | 2 (`GET`/`SET`) | Deferred (100ms flush) | Counter + HDR Histogram |
| `sharded_bucketed` | **Follow-up Addition** | `DashMap` (64 shards) | Configurable `C` | Eager, Shared | Counter (fixed per-op hash) |
| `thread_local_owned` | **Follow-up Addition** | Thread-Local Map | 2 (`GET`/`SET`) | Deferred (100ms flush) | Counter (owned string key) |

---

## Testing, Hardening & Benchmarking

### 1. Test Suite
Run unit tests, keyspace backend tests, and full server integration tests:
```bash
cargo test --workspace
```
* **Store Integrity**: Validates concurrent writes and expiration across both `mutex` and `dashmap` stores.
* **Server Integration**: Starts an ephemeral server, connects over TCP, and verifies command execution, pipelining, and error responses.

### 2. Protocol Fuzzing
The RESP parser has been hardened against hostile inputs using `cargo-fuzz` ([fuzz/](fuzz)):
```bash
# Requires Rust nightly
cargo +nightly fuzz run parse_frame
```
The hardened parser has survived 29.3 million continuous fuzzing executions without panic, verifying protection against integer overflow, negative lengths, and unbounded recursive nesting.

### 3. Load Testing (`rustredis-bench`)
The repository includes a custom high-performance benchmarking crate in [benchmarks/](benchmarks):
```bash
# Compile benchmark tool
cargo build --release --manifest-path benchmarks/Cargo.toml

# Execute benchmark run against a running server
cargo run --release --manifest-path benchmarks/Cargo.toml -- \
  --host 127.0.0.1 --port 6379 \
  --concurrency 100,500,1000 \
  --requests 10000 \
  --runs 15 \
  --workload mixed \
  --key-space 10000 \
  --value-size 64 \
  --output-dir results/local_test
```

---

# Path B: Understand the Research

RustRedis was developed alongside an empirical systems paper investigating the microarchitectural and concurrency costs of in-memory metrics instrumentation:

> **"Cardinality Before Contention: Decomposing Per-Command Observability Overhead on Concurrent Thread-Pool Key-Value Stores"**  
> Full manuscript: **[docs/final_paper_v27.md](docs/final_paper_v27.md)**  
> Comprehensive empirical data & tables: **[docs/research.md](docs/research.md)**  
> Archived Artifact: DOI [10.5281/zenodo.23295261](https://doi.org/10.5281/zenodo.23295261) (tag `v1.0-paper`)

```
========================================================================================
                                    RESEARCH SUMMARY
========================================================================================
 Research Question  │ What does per-command telemetry actually cost on a concurrent
                    │ thread-pool server — and which design decision is the expensive one?
────────────────────┼───────────────────────────────────────────────────────────────────
 Experimental Scale │ 29,520 benchmark trials across 14 dedicated Azure VMs (D4s_v6, D8s_v6)
                    │ 100 to 3,000 concurrent clients; 3 read/write mixes.
────────────────────┼───────────────────────────────────────────────────────────────────
 Core Finding 1     │ CARDINALITY BEATS CONTENTION: Expanding label cardinality costs
                    │ 1.66× to 4.5× more than the throughput saved by eliminating lock
                    │ contention. High-cardinality sharded maps are slower than global locks.
────────────────────┼───────────────────────────────────────────────────────────────────
 Core Finding 2     │ CLIENT LOAD INVARIANCE: Telemetry overhead is flat across 30×
                    │ client concurrency; synchronization costs scale with CPU worker count.
────────────────────┼───────────────────────────────────────────────────────────────────
 Core Finding 3     │ OVERHEAD RIGIDITY: Counter overhead is elastic and absorbed by queueing
                    │ slack at p99; HDR histogram cost is rigid across all percentiles.
────────────────────┼───────────────────────────────────────────────────────────────────
 Core Finding 4     │ BOUNDED CARDINALITY: The cardinality penalty is non-linear and emerges
                    │ only above ~1,000 active entries; low-cardinality label counts are free.
========================================================================================
```

## 1. The Research Question

When engineering telemetry for concurrent servers, backend developers instinctively fear **synchronization contention** (locks, shared atomics) and migrate toward sharded or lock-free data structures. 

This research asks:
1. How does lock contention compare to **metric key cardinality**, **aggregation locality**, and **payload complexity**?
2. How does telemetry overhead scale when a thread-pool server enters real queueing saturation?
3. Does telemetry cost survive into tail latency ($p99$)?

## 2. Experimental Design & Factorial Axes

We evaluate telemetry along four orthogonal design axes:
1. **Synchronization (`SYNC`)**: Global `Mutex` vs. 64-shard concurrent `DashMap`.
2. **Cardinality (`CARD`)**: Low cardinality (2 keys) vs. high cardinality (~10,000 keys).
3. **Aggregation Locality (`DEFER`)**: Eager shared updates vs. deferred thread-local aggregation.
4. **Payload (`PAYLOAD`)**: 64-bit integer counter vs. HDR Histogram (`HdrHistogram`).

### The 6 Main Strategies vs. 8 Implementation Strategies
The main factorial experiment evaluated **6 strategies** (`disabled`, `global_mutex`, `sharded_2key`, `sharded_n`, `thread_local`, `hdr_histogram`). The **2 additional strategies** (`sharded_bucketed` and `thread_local_owned`) were introduced in the implementation to isolate pure map cardinality from string formatting, and pure histogram cost from string allocation. For the full strategy matrix, see [docs/research.md](docs/research.md#22-six-main-experiment-strategies-vs-eight-implementation-strategies).

## 3. Key Findings

* **Cardinality Dominates Synchronization**: Moving from a 2-key label to ~10,000 keys reduces throughput by **0.87 pp** ($95\%$ CI $[0.73, 1.00]$). In contrast, replacing a global mutex with a 64-way sharded map saves only **0.52 pp** ($95\%$ CI $[0.42, 0.63]$). Contention sharding cannot compensate for high-cardinality labeling.
* **Overhead is Invariant to Client Concurrency**: Over a $30\times$ client scaling sweep ($c=100 \to c=3000$), server $p99$ latency degrades by $104\times$ (0.76 ms to 78.85 ms), yet telemetry overhead remains completely flat (fitted slope $< 0.041$ pp/doubling). Telemetry overhead scales with OS worker threads ($2.6\times$ from 4 to 8 workers), not client connections.
* **Overhead Rigidity in Tail Latency**: Counter overheads drop by 40–60% between $p50$ and $p99$ because queueing delay absorbs small CPU delays. However, HDR Histogram overhead remains rigid at ~0.9 pp across throughput, mean, $p50$, and $p99$.
* **Cardinality Cost Appears Above $\sim 10^3$ Keys**: Controlled sweeps with `sharded_bucketed` prove that cardinality overhead is indistinguishable from zero below 1,000 entries, but increases sharply at 2,000 and 13,040 entries.

For full result tables, confidence intervals, and rigidity indices, refer to **[docs/research.md](docs/research.md#6-comprehensive-result-tables)**.

## 4. Statistical Methodology

* **Randomized Multiple Interleaved Trials (RMIT)**: Mitigates environmental drift and virtualization noise by randomizing execution order within repeated measurement blocks.
* **Within-Block Paired Ratios**: All overheads are computed as ratios against the paired `disabled` run within the exact same block.
* **Run-Level Clustered Inference**: 11 independent runs $\times$ 15 blocks = 165 blocks per configuration. To avoid overstating precision, inference clusters across independent VM runs ($N=11$), collapsing blocks to run means and constructing Student's $t$-intervals verified by exact sign tests.

## 5. Limitations & Caveats

* **Co-located Workload**: Client and server ran on the same VM over loopback (`127.0.0.1`) to measure pure CPU and memory overhead without network interface cards masking telemetry costs.
* **Worker Counts**: Verified on 4 vCPU and 8 vCPU instances (`Standard_D4s_v6`, `Standard_D8s_v6`).
* **Keyspace Backend**: Published paper figures utilized the single-mutex keyspace engine (`Db`) as the reference baseline. The sharded engine (`DbDashMap`) was tested to confirm that keyspace contention does not alter collector rankings (see [docs/CONFOUND_CHECK.md](docs/CONFOUND_CHECK.md)).

## 6. Accessing the Paper & Artifacts

| Document / Resource | Description |
|---|---|
| **[docs/research.md](docs/research.md)** | **Consolidated Research Guide**: All datasets, full result tables, scaling slopes, and failure analysis in a single document. |
| **[docs/final_paper_v27.md](docs/final_paper_v27.md)** | **The Manuscript**: Complete submitted paper (*"Cardinality Before Contention"*), section for section and table for table. |
| **[docs/rmit_experiment_protocol.md](docs/rmit_experiment_protocol.md)** | Main RMIT experimental protocol and Azure VM provisioning guide. |
| **[docs/followup_experiment_protocol.md](docs/followup_experiment_protocol.md)** | Axis-separation protocol for `sharded_bucketed` and `thread_local_owned`. |
| **[docs/figures/](docs/figures)** | Publication figures in high-res PNG and vector PDF format. |
| **[CITATION.cff](CITATION.cff)** | Citation metadata for academic publications. |

### Reproducing the Results

To download the raw data from Zenodo and regenerate all numbers, tables, and figures from scratch:

```bash
# 1. Setup Python analysis environment
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt

# 2. Fetch raw datasets (~7 MB, SHA256 verified)
scripts/fetch_data.sh

# 3. Regenerate all paper figures and statistics
python3 scripts/analyze_design_axes.py
python3 scripts/generate_design_axes_figures.py
```
Outputs regenerate byte-identical figures in `docs/figures/` and analysis in `experiments/design_axes_analysis.json`.

---

## Repository Structure

```
.
├── src/                       # RustRedis server implementation
│   ├── bin/server.rs          # Server entry point & Tokio connection loop
│   ├── cmd/                   # Command parsing, execution, and dispatch
│   ├── command_metrics.rs     # The 8 telemetry collectors
│   ├── connection.rs          # Buffered TCP stream reader/writer
│   ├── db.rs                  # Single-mutex keyspace storage engine
│   ├── db_dashmap.rs          # Sharded keyspace storage engine (DashMap)
│   ├── frame.rs               # RESP protocol parser and serializers
│   ├── persistence.rs         # AOF persistence and recovery replay
│   ├── pubsub.rs              # Tokio broadcast-based Pub/Sub
│   └── store.rs               # Storage engine abstraction trait
├── benchmarks/                # rustredis-bench: custom asynchronous load generator
├── docs/                      # Documentation & Research
│   ├── research.md            # Complete consolidated research data and analysis
│   ├── final_paper_v27.md     # Full submitted research paper manuscript
│   ├── figures/               # Vector and PNG publication figures
│   └── *_protocol.md          # Cloud VM provisioning and RMIT execution protocols
├── experiments/               # Analysis results and pooled experimental datasets
├── fuzz/                      # LibFuzzer targets for parser robustness testing
├── scripts/                   # Python automation: RMIT execution, analysis, pooling
└── tests/                     # Server integration and end-to-end tests
```

---

## License

This project is licensed under the [MIT License](LICENSE).
