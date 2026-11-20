# RustRedis

[![CI](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml/badge.svg)](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.23295261.svg)](https://doi.org/10.5281/zenodo.23295261)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

RustRedis is a high-performance, Redis-compatible in-memory key-value server written in Rust (~3,500 LOC). It is built on a multi-threaded asynchronous runtime powered by Tokio, featuring pluggable storage backends, Append-Only File (AOF) persistence, and an interchangeable telemetry subsystem.

The project also serves as the artifact for an empirical study on the concurrency and microarchitectural costs of observability telemetry in thread-pool servers (*"Cardinality Before Contention"*, prepared for *Software: Practice and Experience*). Full empirical datasets and statistical results are documented in **[docs/research.md](docs/research.md)**.

---

## Features

- **Redis Protocol (RESP2) Support**: Robust binary-safe framing for Simple Strings, Errors, Integers, Bulk Strings, Arrays, and Nulls.
- **Asynchronous Multi-threaded Runtime**: Powered by Tokio; client connections run concurrently as lightweight async tasks dispatched across an OS worker thread pool.
- **Pluggable Keyspace Storage**:
  - `mutex`: Single global lock protecting a `HashMap` (baseline reference).
  - `dashmap`: 64-way sharded concurrent hash map (`DashMap`) for scalable multi-threaded reads and writes.
- **AOF Persistence**: Write-ahead logging (WAL) in standard RESP format with configurable sync policies (`Always`, `EverySecond`, `No`) and crash recovery replay.
- **Pub/Sub Engine**: Channel-based publish/subscribe message delivery backed by Tokio broadcast channels.
- **Pluggable Observability Collectors**: 8 interchangeable metrics collectors to measure and evaluate per-command telemetry overhead.
- **Hardened Implementation**: Fuzzed with `cargo-fuzz` (over 29.3 million executions survived without crash).

---

## Quick Start

### Building the Server

Ensure you have Rust 1.75+ installed:

```bash
cargo build --release --bin server
```

### Running the Server

Start RustRedis with default settings (listens on `127.0.0.1:6379`):

```bash
cargo run --release --bin server
```

Output:
```text
INFO RustRedis server listening on 127.0.0.1:6379
INFO Keyspace store: mutex
INFO Command metrics initialized (strategy: sharded_2key)
INFO AOF persistence enabled with EverySecond sync policy (path: appendonly.aof)
INFO Press CTRL+C to shutdown gracefully
```

### Connecting with Redis Clients

Connect using standard clients like `redis-cli`, Python (`redis-py`), Go (`go-redis`), or raw TCP (`nc`, `telnet`):

```bash
# Basic connectivity
redis-cli -p 6379 PING
# PONG

# String key-value with TTL expiration
redis-cli -p 6379 SET session:token "9f8a3c" EX 300
# OK
redis-cli -p 6379 GET session:token
# "9f8a3c"

# List operations
redis-cli -p 6379 LPUSH queue:jobs "job_01" "job_02"
# (integer) 2
redis-cli -p 6379 LRANGE queue:jobs 0 -1
# 1) "job_02"
# 2) "job_01"

# Query live telemetry statistics
redis-cli -p 6379 STATS
redis-cli -p 6379 CMDSTAT
```

---

## Configuration

RustRedis is configured via environment variables:

| Variable | Default | Description |
|---|---|---|
| `RUSTREDIS_BIND` | `127.0.0.1:6379` | Socket address and port to listen on. |
| `RUSTREDIS_STORE` | `mutex` | Keyspace storage engine: `mutex` (global lock) or `dashmap` (64 shards). |
| `RUSTREDIS_METRICS_STRATEGY` | `sharded_2key` | Metrics telemetry collector strategy (one of 8 strategies, see below). |
| `RUSTREDIS_METRICS_CARDINALITY` | `2` | Active key cardinality for the `sharded_bucketed` collector. |
| `RUSTREDIS_DISABLE_AOF` | `false` | Set to `1`, `true`, or `yes` to disable AOF persistence. |
| `RUSTREDIS_AOF_PATH` | `appendonly.aof` | File path for the Append-Only File. |

#### Example: High-Throughput Sharded Mode
```bash
RUSTREDIS_STORE=dashmap RUSTREDIS_METRICS_STRATEGY=thread_local cargo run --release --bin server
```

---

## System Architecture

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

### 1. RESP Protocol & Network Framing
- Implemented in [src/frame.rs](src/frame.rs) and [src/connection.rs](src/connection.rs).
- Parses standard Redis RESP2 frames with strict validation: negative length rejection, nesting limits, integer bounds checking, and memory allocation protections.
- Connections use buffered streaming over `tokio::net::TcpStream` with `bytes::BytesMut`, minimizing allocations and flushing pending replies efficiently under pipelined requests.

### 2. Tokio Asynchronous Concurrency Model
- Implemented in [src/bin/server.rs](src/bin/server.rs).
- Uses `#[tokio::main(flavor = "multi_thread")]`. Each accepted TCP connection is handled in an independent asynchronous task.
- Worker threads match host CPU cores (vCPUs), decoupling network connection concurrency from OS thread execution. Client connection counts do not cause thread oversubscription inside the server runtime.

### 3. Storage Engines (`Store` Trait)
- Defined in [src/store.rs](src/store.rs):
  - **`Db` ([src/db.rs](src/db.rs))**: Uses `Arc<Mutex<Shared>>` wrapping a `HashMap`. Provides predictable, strict single-lock serialization.
  - **`DbDashMap` ([src/db_dashmap.rs](src/db_dashmap.rs))**: Uses `DashMap` partitioned across 64 independent shards to reduce write contention under high concurrency.
- Expiration: Passive/lazy TTL evaluation on key access across all data types.

### 4. Append-Only File (AOF) Persistence
- Implemented in [src/persistence.rs](src/persistence.rs).
- Serializes write commands into RESP frames and appends them to disk.
- Sync policies:
  - `Always`: Calls `fsync()` after every write command.
  - `EverySecond` (default): Asynchronously flushes the file descriptor every second via a dedicated background Tokio task.
  - `No`: Leaves synchronization timing to the operating system.
- Crash recovery: Upon startup, `Aof::load()` reads the AOF log, parsing and replaying recorded commands to reconstruct the in-memory keyspace before serving requests.

### 5. Pub/Sub Engine
- Implemented in [src/pubsub.rs](src/pubsub.rs).
- Manages real-time broadcast channels with `tokio::sync::broadcast` (bounded at 1,024 messages per channel). Unused channels are automatically pruned when subscriber counts drop to zero.

---

## Supported Redis Commands

Implemented in [src/cmd/mod.rs](src/cmd/mod.rs):

| Category | Commands | Description |
|---|---|---|
| **Server & Connection** | `PING [msg]` | Tests server connectivity; echoes optional message. |
| | `ECHO msg` | Returns the input message. |
| | `STATS` | Returns process uptime, connected clients, operations count, and AOF sync stats. |
| | `CMDSTAT` | Returns per-command call counts and execution times from the active metrics collector. |
| **Strings & Keyspace** | `GET key` | Retrieves the string value of a key. |
| | `SET key val [EX sec]` | Sets key to value with optional TTL expiration in seconds. |
| | `DEL key [key...]` | Deletes one or more keys. |
| | `EXISTS key` | Returns 1 if key exists, 0 otherwise. |
| | `TYPE key` | Returns the data type (`string`, `list`, `set`, `hash`, `none`). |
| | `DBSIZE` | Returns the count of keys in the database. |
| | `FLUSHDB` | Deletes all keys from the database. |
| | `KEYS pattern` | Returns all keys matching glob-style pattern (e.g. `user:*`). |
| **Lists** | `LPUSH key val...` | Prepends one or more elements to a list. |
| | `RPUSH key val...` | Appends one or more elements to a list. |
| | `LPOP key` | Pops element from the left of a list. |
| | `RPOP key` | Pops element from the right of a list. |
| | `LRANGE key start stop`| Returns range of elements (supports negative indices). |
| | `LLEN key` | Returns length of a list. |
| **Sets** | `SADD key mem...` | Adds members to a set. |
| | `SREM key mem...` | Removes members from a set. |
| | `SMEMBERS key` | Returns all members in a set. |
| | `SISMEMBER key mem` | Checks if a member exists in a set. |
| | `SCARD key` | Returns cardinality of a set. |
| **Hashes** | `HSET key field val` | Sets field value in a hash. |
| | `HGET key field` | Retrieves field value from a hash. |
| | `HGETALL key` | Returns all fields and values in a hash. |
| | `HDEL key field...` | Deletes fields from a hash. |
| | `HEXISTS key field` | Checks if field exists in a hash. |
| | `HLEN key` | Returns count of fields in a hash. |
| **Pub/Sub** | `PUBLISH channel msg` | Publishes message to a channel; returns subscriber count. |

---

## Telemetry Collectors: The 8 Implementation Strategies

RustRedis features 8 interchangeable metrics collectors in [src/command_metrics.rs](src/command_metrics.rs) to examine how observability designs impact request throughput and latency.

### ⚠️ Note on Research Strategies vs. Implementation Strategies
- **6 Strategies in the Main Paper Experiment**: The primary empirical study evaluated **six strategies** (`disabled`, `global_mutex`, `sharded_2key`, `sharded_n`, `thread_local`, `hdr_histogram`) configured along 4 factorial design axes.
- **8 Strategies in the Final Implementation**: To isolate potential confounds (separating map entry count from string hashing/formatting, and separating histogram tracking from owned-string allocation), **two auxiliary strategies (`sharded_bucketed` and `thread_local_owned`) were introduced** in the follow-up axis-separation study.

All eight collectors are available via `RUSTREDIS_METRICS_STRATEGY`:

| Strategy Name | Scope | Synchronization | Cardinality | Aggregation | Payload Recorded |
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

## Testing & Hardening

### 1. Test Suite
Run unit tests, keyspace backend tests, and server integration tests:
```bash
cargo test --workspace
```

### 2. Protocol Fuzzing
Hardened against malicious or malformed inputs using `cargo-fuzz` ([fuzz/](fuzz)):
```bash
cargo +nightly fuzz run parse_frame
```
Survives tens of millions of continuous executions with zero panics.

### 3. Benchmark Harness (`rustredis-bench`)
The custom load generator in [benchmarks/](benchmarks) can be used to run throughput and latency sweeps:
```bash
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

## Research & Academic Artifact

RustRedis was developed alongside an empirical study on in-memory key-value server instrumentation:

> **"Cardinality Before Contention: Decomposing Per-Command Observability Overhead on Concurrent Thread-Pool Key-Value Stores"**  
> Prepared for *Software: Practice and Experience*.  
> Full manuscript: **[docs/final_paper_v27.md](docs/final_paper_v27.md)**  
> Complete empirical data & tables: **[docs/research.md](docs/research.md)**  
> Archived Artifact: DOI [10.5281/zenodo.23295261](https://doi.org/10.5281/zenodo.23295261) (tag `v1.0-paper`)

### Summary of Research Findings

1. **Cardinality Beats Contention**: High-cardinality metric labels (~10,000 keys) cost **0.87 pp** of throughput, whereas eliminating lock contention via sharding saves only **0.52 pp**. A sharded map with high-cardinality labels is measurably *slower* than a single global lock with low-cardinality labels.
2. **Client Load Invariance**: Over a 30× client concurrency sweep (100 to 3,000 clients), telemetry overhead remains flat even as server p99 latency degrades by 104×. Telemetry synchronization costs scale with OS worker threads, not client connections.
3. **Overhead Rigidity in Tail Latency**: Counter overheads drop by 40–60% between p50 and p99 as queueing slack absorbs minor delays. In contrast, HDR Histogram payload costs remain rigid (~0.9 pp) across all percentiles.
4. **Bounded Cardinality**: Below ~1,000 entries, label cardinality costs are statistically indistinguishable from zero; the penalty emerges primarily at higher key spaces.

For full result tables, confidence intervals, scaling slopes, and failure analyses, see **[docs/research.md](docs/research.md)**.

### Reproducing the Paper Results

```bash
# 1. Setup Python analysis environment
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt

# 2. Fetch raw datasets from Zenodo (~7 MB, SHA256 verified)
scripts/fetch_data.sh

# 3. Regenerate all tables and publication figures
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
│   ├── research.md            # Comprehensive research data, tables, and methodology
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
