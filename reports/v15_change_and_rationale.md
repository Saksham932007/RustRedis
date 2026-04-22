# v15 Technical Change Log and Rationale

## 1. Purpose

This document records, in one place, what was changed, why those changes were made, and how each claim was validated for final paper completion (v15).

This is an engineering evidence document, not manuscript prose.

## 2. Scope of Changes

The v15 validation covered five technical areas:

1. Causal attribution: code change vs workload parameterization
2. DashMap implementation behavior
3. Shard distribution behavior under Sharded-2key vs Sharded-N
4. Figure dataset integrity and consistency
5. Zenodo package readiness

## 3. What Changed

### 3.1 Metrics Architecture and Keying Model

Primary files:

- `src/command_metrics.rs`
- `src/bin/server.rs`
- `src/cmd/mod.rs`

Changes introduced:

1. Metrics strategy model evolved from a single sharded mode into explicit strategy variants:
   - `Sharded2Key`
   - `ShardedN`
   - `HdrHistogram`
2. Recording API changed from command-only recording to command + key-hint recording:
   - from `record(cmd_name, duration_us)`
   - to `record(cmd_name, key_hint, duration_us)`
3. Shard count became explicit and fixed (`64`) for metrics DashMaps.
4. Added shard-level diagnostics in `CMDSTAT` output:
   - `sharded_2key_get_shard`
   - `sharded_2key_set_shard`
   - per-shard call counters
   - `sharded_n_nonempty_shards`
   - per-shard key/call counters
5. Sharded-N path records by logical key and uses `String` keys (via `metric_key.to_string()`).

Why these changes were made:

- To separate low-cardinality (command-name) contention from high-cardinality (logical-key) contention.
- To make shard pressure visible and auditable in runtime telemetry.
- To support causal analysis of strategy behavior under high concurrency.

### 3.2 Benchmark Measurement Semantics

Primary file:

- `benchmarks/src/main.rs`

Changes introduced:

1. Added explicit warmup phase (10% of per-client requests).
2. Added barrier synchronization so measured timing starts only after warmup completes.
3. Changed request semantics so each client executes full `requests_per_client` (instead of earlier per-concurrency splitting).
4. Added explicit warmup/measured fields in benchmark outputs:
   - `warmup_ops_per_client`
   - `measured_ops_per_client`

Why these changes were made:

- To reduce transient startup effects in measured latency/throughput.
- To keep per-client load definition consistent and explicit.
- To improve reproducibility of matrix comparisons and statistical interpretation.

### 3.3 Experiment Pipeline and Dataset Packaging

Primary files:

- `benchmarks/run_final_experiment_v12.py`
- `benchmarks/generate_final_experiment_v12_dataset.py`
- `final_experiment_v12.json`
- `experiment_results_v12/*`

Changes introduced:

1. v12 pipeline formalized raw + aggregated dataset generation.
2. Figure datasets exported in structured form:
   - throughput vs concurrency
   - p99 vs concurrency
   - CV vs concurrency
   - distribution at 500 clients
3. System-validation artifacts integrated into dataset generation.
4. Metadata/config artifacts created for reproducibility and deposition.

Why these changes were made:

- To produce a publication-ready data package with traceable lineage.
- To eliminate ambiguity between raw runs, aggregated statistics, and figure inputs.
- To satisfy reproducibility and archive requirements.

## 4. Why Causal Attribution is `CODE_CHANGE`

Final causal classification: `CODE_CHANGE`.

Reasoning:

1. The metrics implementation changed materially (strategy model, keying route, shard diagnostics, explicit shard amount).
2. The benchmark workload measurement semantics also changed (warmup + request accounting semantics).
3. Therefore, observed performance differences cannot be attributed to workload scaling alone.

Mechanism summary:

- Workload-side changes affect what is included in the measured window (warmup exclusion) and request accounting.
- Metrics-side changes affect contention and allocation behavior across different strategy/keying paths.

## 5. Validation Performed

### 5.1 Commit and Diff Audit

Commits audited in detail:

- `be472595525c361656ca88f3ed2d2861105083f0`
- `624cb56a4e3e9dc0f7fde6cc047085a330a28d21`
- `0c6384c1a22fec745d3e058bd2d37471a10c5b01`
- `1a605ed40b6870e6a16be0ed990b071b268985a2`

Outcome:

- Confirmed material code and workload instrumentation changes between earlier and current states.

### 5.2 Repository Validation (Build/Execution)

Validation actions:

1. Built release server binary.
2. Built release benchmark binary.
3. Ran fresh shard validation harness at concurrency 500 for:
   - `sharded_2key`
   - `sharded_n`

Fresh run artifact directory:

- `results/system_validation_v15/20260422_124008/`

Outcome:

- Build and execution completed successfully.

### 5.3 DashMap Implementation Validation

Evidence sources:

- `src/command_metrics.rs`
- DashMap crate source (`dashmap-5.5.3`) from local cargo registry

Validated points:

1. Metrics shard count configured to `64`.
2. DashMap default hasher is `std::collections::hash_map::RandomState` in this setup.
3. Sharded-2key uses `&'static str` metric keys.
4. DashMap internals are shard-based with per-shard `RwLock`.

### 5.4 Shard Distribution Validation

#### Legacy system validation set (used by v12 dataset)

From:

- `experiment_results_v12/system_validation/20260421_170502/`

Observed:

- Sharded-2key active shards: `[6, 37]`
- Calls at active shards: `[250222, 249778]`
- Sharded-N total active shards: `64`
- Sharded-N call range: `min=6174`, `max=9118`

#### Fresh rerun on current code

From:

- `results/system_validation_v15/20260422_124008/`

Observed:

- Sharded-2key active shards (GET/SET): `[16, 25]`
- Calls at active shards: `[250422, 249578]`
- Sharded-N total active shards: `64`
- Sharded-N call range: `min=6578`, `max=9881`

Interpretation:

- Sharded-2key maps to a very small active shard subset (command-key cardinality collapse).
- Sharded-N spreads traffic across all shards (high key cardinality distribution).

### 5.5 Figure Data Validation

Validated datasets:

1. Throughput vs concurrency
2. p99 vs concurrency
3. CV vs concurrency
4. Distribution at 500 clients

Checks run:

1. Non-negative latency metrics in raw data.
2. CI recomputation from raw data matches aggregated CSV.
3. Naming consistency across raw, aggregated, and figure dataset blocks.

Results:

- `latency_non_negative = true`
- `ci_consistency = true`
- `naming_consistent = true`
- `issues = []`

Dataset scale observed during validation:

- Raw rows: `1440`
- Aggregated rows: `48`

### 5.6 Zenodo Package Readiness

Required package components were verified present, including:

- `experiment_results_v12/raw_data.csv`
- `experiment_results_v12/aggregated_data.csv`
- `experiment_results_v12/run_data/`
- `experiment_results_v12/system_validation/`
- `experiment_results_v12/metadata.json`
- `experiment_results_v12/final_experiment_config.json`
- `final_experiment_v12.json`
- `benchmarks/run_final_experiment_v12.py`
- `benchmarks/generate_final_experiment_v12_dataset.py`
- `benchmarks/generate_final_experiment_report.py`

Result:

- `ready_for_upload = true`

## 6. Important Provenance Finding

A provenance inconsistency was detected:

- `experiment_results_v12/metadata.json` reports commit `be472595...`
- But source at that commit does not include the later strategy and warmup semantics present in the generated artifacts.

Impact:

- The dataset is valid as an artifact set, but the commit hash in metadata does not fully represent the effective code state used for all generated outputs.

Recommended action:

1. Regenerate metadata with the exact producing commit hash, or
2. Add an explicit provenance note in the deposition README describing the producing commit window.

## 7. Final Outcome Snapshot

The v15 evidence status is:

- Causal validation: `CODE_CHANGE`
- DashMap validation: complete
- Shard distribution validation: complete
- Figure validation: complete
- Zenodo package validation: complete

Canonical machine-readable evidence file:

- `v15_evidence.json`

## 8. Traceability Artifacts

Primary artifacts to review:

- `v15_evidence.json`
- `results/system_validation_v15/20260422_124008/`
- `experiment_results_v12/system_validation/20260421_170502/`
- `experiment_results_v12/raw_data.csv`
- `experiment_results_v12/aggregated_data.csv`
- `final_experiment_v12.json`

---

Prepared for final paper completion support (v15 technical validation).
