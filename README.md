# RustRedis

[![CI](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml/badge.svg)](https://github.com/Saksham932007/RustRedis/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.23295261.svg)](https://doi.org/10.5281/zenodo.23295261)

RustRedis is an experimental Redis-compatible in-memory key-value server in Rust, built
to answer one research question: **what does per-command observability instrumentation
actually cost on a thread-pool concurrent server, and which design decision inside
"instrumentation" is the expensive one?** It speaks RESP, so `redis-cli` works against
it. It supports strings (with `EX` expiry), lists, sets, hashes, pub/sub publishing,
binary-safe keys, and optional append-only-file persistence.

## Run it

```bash
cargo run --release --bin server                  # listens on 127.0.0.1:6379
redis-cli -p 6379 set greeting hello              # or: printf '*1\r\n$4\r\nPING\r\n' | nc 127.0.0.1 6379
redis-cli -p 6379 get greeting
cargo test --workspace                            # unit + end-to-end tests
```

Configuration is by environment variable:

| Variable | Default | Meaning |
|---|---|---|
| `RUSTREDIS_BIND` | `127.0.0.1:6379` | Listen address |
| `RUSTREDIS_STORE` | `mutex` | Keyspace backend: `mutex` (one global lock) or `dashmap` (sharded) |
| `RUSTREDIS_METRICS_STRATEGY` | `sharded_2key` | Per-command metrics collector (one of eight; unknown names are an error) |
| `RUSTREDIS_METRICS_CARDINALITY` | `2` | Entry count for the `sharded_bucketed` collector |
| `RUSTREDIS_DISABLE_AOF` | off | `1`/`true`/`yes` disables the append-only file |
| `RUSTREDIS_AOF_PATH` | `appendonly.aof` | AOF location |

## Headline result

Per-command metrics collectors are built so adjacent pairs differ along exactly one
design axis (synchronization, key cardinality, aggregation locality, payload). Across
29,520 benchmark runs on fourteen dedicated Azure VMs, **moving the metric key from a
2-entry command-name label to a ~10,000-entry data-key label costs 0.87 pp of
throughput, 1.66x more than the 0.52 pp that replacing a global mutex with a 64-way
sharded map saves** ("cardinality before contention").

Full results: [docs/RESULTS.md](docs/RESULTS.md). Manuscript:
[docs/final_paper_v27.md](docs/final_paper_v27.md). Method: Randomized Multiple
Interleaved Trials (Abedi, Heard & Brecht 2015), not invented here.

## Known limitations

- **Keyspace lock.** The default `mutex` store serialises every GET and SET behind one
  lock, so requests are partly serialised before they reach the metrics collector.
  That could compress the synchronization-axis differences. A sharded `dashmap` store
  is now selectable so the comparison can be repeated; see
  [docs/CONFOUND_CHECK.md](docs/CONFOUND_CHECK.md) for what a quick local check does
  and does not show. The published results were measured on the `mutex` store.
- **Loopback.** Client and server share one machine, so this is not a network-bound
  measurement.
- Further caveats are listed in [docs/RESULTS.md](docs/RESULTS.md#caveats).

## Reproducing the paper

```bash
# 0. Analysis dependencies (Python 3.10+)
python3 -m venv .venv && . .venv/bin/activate && pip install -r requirements.txt

# 1. Build
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml

# 2. Main RMIT experiment (read docs/rmit_experiment_protocol.md first)
python3 scripts/run_rmit_experiment.py --output-dir experiments/output
python3 scripts/analyze_rmit_results.py --input experiments/output/raw_data_rmit.csv

# 3. Axis-separation batch (docs/followup_experiment_protocol.md)
python3 scripts/run_rmit_experiment.py \
  --strategies all --cardinalities 1,10,100,1000,10000 \
  --runs 15 --concurrency 100,1000,3000 --workloads mixed \
  --key-space 10000 --seed 2001 --output-dir experiments/output_followup

# 4. Pool runs and regenerate every number and figure in the paper
python3 scripts/combine_iterations.py --input-dir <runs-dir> --output-dir <runs-dir>/pooled
python3 scripts/analyze_design_axes.py              # -> experiments/design_axes_analysis.json
python3 scripts/generate_design_axes_figures.py     # -> docs/figures/*.{png,pdf}
```

The analysis commands run against the committed data as-is: they need no cloud
VM and regenerate every table, figure and quoted statistic in the paper. Their
outputs are byte-identical on re-run, so a successful reproduction from a fresh
clone leaves `git status` clean.

Both protocol docs include a manual machine-control checklist (power, sleep, background
processes) that materially affects result quality, plus the Azure VM path. Run the
documented smoke test and read its per-run throughput numbers with your own eyes before
committing hours to a full run.

Server and one-off benchmark by hand:

```bash
RUSTREDIS_METRICS_STRATEGY=sharded_bucketed RUSTREDIS_METRICS_CARDINALITY=1000 \
  cargo run --release --bin server

cargo run --release --manifest-path benchmarks/Cargo.toml -- \
  --host 127.0.0.1 --port 6379 \
  --concurrency 100,500,1000 --requests 1000 --runs 30 \
  --workload mixed --key-space 10000 --value-size 64 \
  --output-dir results/manual_run
```

## Repository layout

```
.
├── src/                       Server (Rust): commands, storage backends, metrics collectors
├── benchmarks/                Load-generator crate (rustredis-bench)
├── scripts/                   Experiment runner, analysis, pooling, figures
├── experiments/               Azure VM datasets (raw CSV + analysis JSON)
│   ├── azure_d4s_v6/          D4s_v6, single run
│   ├── azure_d8s_v6/          D8s_v6: run_00..run_10 + pooled/ (11 runs)
│   └── azure_d8s_v6_followup/ D8s_v6 axis separation: run_01..run_08 + pooled/
├── docs/                      Manuscript, protocols, design notes, figures
├── submission/                Cover letter and pre-submission checklist
├── archive/                   Superseded drafts and their figures
├── CITATION.cff               How to cite the artifact and the paper
├── requirements.txt           Python deps for the analysis and figure scripts
└── LICENSE
```

## Documentation

- [docs/final_paper_v27.md](docs/final_paper_v27.md) — **"Cardinality Before
  Contention"**, the manuscript. Every table, figure and quoted statistic
  regenerates from committed artifacts; see its Appendix A.
- [docs/README.md](docs/README.md) — index of the documentation.
- [submission/](submission) — cover letter and pre-submission checklist.
- [archive/](archive) — superseded working material, including an earlier
  measurement-study draft of the same data. Nothing there is cited by the manuscript.
- [docs/rmit_experiment_protocol.md](docs/rmit_experiment_protocol.md) — main protocol
  and Azure VM setup.
- [docs/followup_experiment_protocol.md](docs/followup_experiment_protocol.md) —
  axis-separation batch protocol.
- [docs/system-design.md](docs/system-design.md),
  [docs/failure-analysis.md](docs/failure-analysis.md),
  [docs/related_work_notes.md](docs/related_work_notes.md)
- [docs/figures/](docs/figures) — paper figures (axis decomposition, scaling, rigidity,
  cardinality curve), each as a 200-dpi PNG and a vector PDF for submission.

## Architecture overview

1. TCP listener accepts client connections.
2. Tokio multi-threaded runtime, one task per connection, parses RESP frames.
3. Command executor operates on shared DB state.
4. Optional persistence appends to AOF.
5. Telemetry path updates command metrics via the selected collector.

Worker count is the Tokio default, so **the number of OS threads that can concurrently
enter the metrics path equals the vCPU count** regardless of client count — the property
finding 2 exploits.

Core modules:

- [src/bin/server.rs](src/bin/server.rs): server entry point
- [src/cmd/mod.rs](src/cmd/mod.rs): command parsing/execution
- [src/db.rs](src/db.rs): mutex-backed DB
- [src/db_dashmap.rs](src/db_dashmap.rs): sharded DB backend
- [src/connection.rs](src/connection.rs): network I/O
- [src/frame.rs](src/frame.rs): RESP framing
- [src/persistence.rs](src/persistence.rs): AOF persistence
- [src/command_metrics.rs](src/command_metrics.rs): the eight metrics collectors
- [src/metrics.rs](src/metrics.rs): process/system counters
- [src/pubsub.rs](src/pubsub.rs): pub/sub manager

## Citing this work

See [CITATION.cff](CITATION.cff). The release corresponding to the manuscript is
tagged `v1.0-paper`, archived on Zenodo with DOI
[10.5281/zenodo.23295261](https://doi.org/10.5281/zenodo.23295261).

## License

MIT.
