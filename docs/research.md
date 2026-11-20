# RustRedis Research: Empirical Study on Observability Overhead

This document consolidates the complete empirical research data, experimental methodology, statistical framework, results, and reproduction instructions for the study:

> **"Cardinality Before Contention: Decomposing Per-Command Observability Overhead on Concurrent Thread-Pool Key-Value Stores"**  
> Prepared for submission to *Software: Practice and Experience*.  
> Manuscript: [docs/final_paper_v27.md](final_paper_v27.md)  
> Tagged artifact: `v1.0-paper` | DOI: [10.5281/zenodo.23295261](https://doi.org/10.5281/zenodo.23295261)

---

## 1. Research Question

High-throughput, concurrent in-memory services rely on per-command telemetry for SLO monitoring, capacity planning, and debugging. However, telemetry introduces overhead into the request hot path. This research addresses the fundamental question:

> **What does per-command observability instrumentation actually cost on a thread-pool concurrent server — and which design decision inside "instrumentation" is the expensive one?**

Software engineers commonly assume lock contention is the primary telemetry bottleneck and attempt to eliminate it using sharded maps or atomic operations. This study isolates and quantifies the true drivers of telemetry overhead across concurrent workloads.

---

## 2. Experimental Design

### 2.1 Factorial Decomposition (The 4 Design Axes)

Rather than benchmarking ad-hoc metric libraries, we decompose per-command metrics collection into **four orthogonal design axes**:

1. **Synchronization Discipline (`SYNC`)**: Single shared mutex vs. concurrent sharded map (64 shards).
2. **Metric Key Cardinality (`CARD`)**: Low cardinality (2 keys: command name) vs. high cardinality (~10,000 keys: individual data keys).
3. **Aggregation Locality (`DEFER`)**: Eager shared update vs. deferred thread-local counter with periodic flush.
4. **Per-Sample Payload (`PAYLOAD`)**: Plain 64-bit integer counter vs. HDR Histogram (`HdrHistogram`, recording latency distribution).

Timing (`Instant::now()` / `elapsed()`) and key hint extraction run identically for all strategies (including `disabled`), ensuring these fixed overheads cancel in paired comparisons.

```
       [Request Path]
             │
             ├── (1) SYNC: GlobalMutex vs. Sharded2Key
             │
             ├── (2) CARD: Sharded2Key (2 keys) vs. ShardedN (~10,000 keys)
             │
             ├── (3) DEFER: Sharded2Key (eager) vs. ThreadLocal (deferred)
             │
             └── (4) PAYLOAD: ThreadLocal (counter) vs. HdrHistogram (HDR histogram)
```

### 2.2 Six Main Experiment Strategies vs. Eight Implementation Strategies

A crucial distinction in this codebase is between the **six strategies used in the main factorial experiment** and the **eight strategies available in the final implementation**:

| Strategy Name | Scope | Synchronization | Cardinality | Aggregation | Payload |
|---|---|---|---:|---|---|
| `disabled` | Main & Final | — | 0 | — | — |
| `global_mutex` | Main & Final | Single global `Mutex<HashMap>` | 2 | eager, shared | Counter |
| `sharded_2key` | Main & Final | `DashMap` (64 shards) | 2 | eager, shared | Counter |
| `sharded_n` | Main & Final | `DashMap` (64 shards) | ~10,000 (data key) | eager, shared | Counter |
| `thread_local` | Main & Final | Thread-local map + periodic flush | 2 | deferred, private | Counter |
| `hdr_histogram` | Main & Final | Thread-local map + periodic flush | 2 | deferred, private | Counter + HDR Histogram |
| `sharded_bucketed` | **Follow-up Addition** | `DashMap` (64 shards) | Configurable `C` | eager, shared | Counter |
| `thread_local_owned` | **Follow-up Addition** | Thread-local map + periodic flush | 2 | deferred, private | Counter (owned string key) |

#### Why two additional strategies were added in the implementation:
In the initial 6-strategy factorial design, two pairs of axes possessed potential confounding factors:
1. **`sharded_n` vs. `sharded_2key`**: `sharded_n` introduced string allocation and hashing on every operation alongside live map size. To isolate *pure map cardinality* from per-op allocation/hashing, `sharded_bucketed` was built. It hashes `"<CMD>#<fnv1a(key) mod C>"` where per-op string construction is identical regardless of `C`, allowing a pure cardinality sweep.
2. **`hdr_histogram` vs. `thread_local`**: `hdr_histogram` used owned `String` labels while `thread_local` used borrowed string slices (`&'static str`). To isolate *pure histogram payload* cost, `thread_local_owned` was built as a line-for-line twin of `hdr_histogram` with the histogram replaced by an atomic counter.

Thus, the final implementation contains **8 interchangeable collectors** in [src/command_metrics.rs](../src/command_metrics.rs).

### 2.3 The Four Contrasts

Contrasts are computed against shared reference strategies, allowing arbitrary comparisons via arithmetic:

| Axis | Contrast Formula | D8s_v6 (8 workers) [95% CI] | D4s_v6 (4 workers) [95% CI] |
|---|---|---|---|
| **CARD** | `sharded_n` − `sharded_2key` | **0.87 [0.73, 1.00]** pp | **0.79 [0.62, 0.96]** pp |
| **PAYLOAD** | `hdr_histogram` − `thread_local` | **0.89 [0.80, 0.97]** pp | **1.00 [0.71, 1.28]** pp |
| **SYNC** | `global_mutex` − `sharded_2key` | **0.52 [0.42, 0.63]** pp | **0.18 [−0.02, 0.38]** pp |
| **DEFER** | `sharded_2key` − `thread_local` | **0.31 [0.21, 0.41]** pp | **0.26 [−0.00, 0.51]** pp |

*(Overhead figures represent percentage points (pp) of throughput reduction compared to the `disabled` baseline; all contrasts held in 11/11 runs.)*

Notice that `CARD − SYNC` algebraically equals `sharded_n − global_mutex`, providing a direct empirical test of cardinality vs. lock contention.

---

## 3. Experimental Datasets & Infrastructure

All experiments were executed on dedicated Azure Virtual Machines provisioned solely for the benchmarks and deleted immediately afterward. All instances reported the identical **Intel Xeon Platinum 8573C** processor microarchitecture.

| Dataset | Instance Specs | Configuration & Scale | Total Runs |
|---|---|---|---:|
| [`azure_d4s_v6`](../experiments/azure_d4s_v6) | `Standard_D4s_v6`, 4 vCPU / 16 GB, Central India | 6 strategies × 8 concurrency levels (100–1000) × 30 blocks, mixed workload | 1,440 |
| [`azure_d8s_v6/pooled`](../experiments/azure_d8s_v6/pooled) | `Standard_D8s_v6`, 8 vCPU / 32 GB, **5 VMs across 3 regions** | 6 strategies × 8 concurrency levels (100–3000) × 3 workloads × 15 blocks, **× 11 independent runs** | 23,760 |
| [`azure_d8s_v6_followup/pooled`](../experiments/azure_d8s_v6_followup/pooled) | `Standard_D8s_v6`, **8 VMs across 3 regions** | 12 configs (including 5-point cardinality sweep) × 3 concurrency × 15 blocks, **× 8 independent runs** | 4,320 |
| **Total** | | | **29,520** |

---

## 4. Statistical Methodology

### 4.1 Randomized Multiple Interleaved Trials (RMIT)
To neutralize temporal confounds, thermal throttling, and multi-tenant virtualization noise, trials are randomized and interleaved (Abedi, Heard & Brecht 2015; Abedi & Brecht, ICPE 2017). Each block contains a randomized ordering of all candidate strategies alongside a paired `disabled` baseline run.

### 4.2 Paired Within-Block Ratios
All overhead numbers are computed as **paired within-block ratios**:
$$\text{Overhead} = \frac{\text{Throughput}_{\text{disabled}} - \text{Throughput}_{\text{strategy}}}{\text{Throughput}_{\text{disabled}}}$$
Because both runs execute within minutes of each other under identical machine state, paired ratios eliminate global performance shifts. For example, during follow-up testing, VM `run_06` exhibited a ~30% hardware degradation across all configurations, but within-block paired ratios shifted by less than 0.05 pp.

### 4.3 Cluster-Robust Run-Level Inference
Pooling 11 independent runs of 15 blocks yields 165 blocks per configuration cell. Treating all 165 blocks as independent samples overstates statistical precision by ~20%. Instead, **inference clusters on independent runs**:
1. Within-run blocks are collapsed to a single mean per run.
2. Student's *t*-confidence intervals (95%) are constructed over the run means ($N = 11$ or $N = 8$).
3. All directional claims are verified with an exact non-parametric sign test.

---

## 5. Key Research Findings

### Finding 1: Cardinality Beats Contention
Moving metric key cardinality from 2 keys (command name) to ~10,000 keys (individual data keys) costs **0.87 pp** of throughput. In contrast, replacing a global mutex with a 64-way sharded concurrent map saves only **0.52 pp**. 

* Cardinality cost is **1.66× larger** than synchronization savings on 8-core VMs (gap: 0.34 pp, 95% CI [0.19, 0.50], 10/11 runs, sign test $p = 0.012$).
* On 4-core VMs (`D4s_v6`), cardinality cost is **4.5× larger** than synchronization savings (0.79 pp vs. 0.18 pp).
* **Implication**: A sharded "scalable" metrics map with high-cardinality labels is measurably *slower* than a single global lock with low-cardinality labels.

### Finding 2: The Axes Obey Different Scaling Laws
Over a **30× increase in client concurrency** (100 to 3,000 concurrent clients), telemetry overhead remains completely flat:
* The largest fitted slope across client doublings is 0.041 pp/doubling (95% CI spans zero).
* The server experiences real saturation: $p99$ latency spikes from 0.76 ms to 78.85 ms ($104\times$), yet telemetry overhead does not amplify.
* The synchronization axis (`SYNC`) scales strictly with **worker threads** (2.6× increase from 4 to 8 workers), not client connections.

### Finding 3: Overhead Rigidity (Elastic Counters vs. Rigid Histograms)
Overhead exhibits distinct distribution behaviors across percentiles:
* **Counter costs are elastic**: Between 38% and 58% of median counter overhead is absorbed by queueing slack at the 99th percentile ($p99$).
* **Histogram costs are rigid**: HDR Histogram overhead remains virtually constant at 0.89–0.94 pp across throughput, mean, $p50$, and $p99$.
* **Rank Reversal**: On throughput, `hdr_histogram` vs. `global_mutex` is unresolved ($p = 0.23$). However, at $p50$, `hdr_histogram` is cheaper ($p = 0.001$), while at $p99$, it is significantly more expensive ($p = 0.001$).

### Finding 4: Cardinality Threshold at $\sim 10^3$
Sweep experiments using `sharded_bucketed` confirm that the cardinality penalty is real and driven by map size, but is non-linear:
* Below $\sim 1,000$ entries, cardinality overhead is **statistically indistinguishable from zero**.
* The penalty appears and grows between 2,000 and 13,040 entries.
* **Implication**: Reducing metric cardinality from 50 to 5 labels yields no measurable performance improvement; optimizations only matter above thousands of active keys.

---

## 6. Comprehensive Result Tables

### 6.1 Cost by Budget Metric (Pooled `azure_d8s_v6`, % vs. `disabled`, 95% CI)

| Strategy | Throughput Reduction | Mean Latency Increase | $p50$ Latency Increase | $p99$ Latency Increase |
|---|---|---|---|---|
| `thread_local` | 0.53 [0.42, 0.65] % | 0.62 [0.50, 0.74] % | 0.72 [0.58, 0.85] % | 0.43 [0.22, 0.64] % |
| `sharded_2key` | 0.84 [0.77, 0.92] % | 0.94 [0.86, 1.01] % | 1.13 [1.03, 1.23] % | 0.54 [0.31, 0.77] % |
| `global_mutex` | 1.37 [1.27, 1.46] % | 1.49 [1.39, 1.58] % | 1.79 [1.71, 1.88] % | 0.78 [0.54, 1.02] % |
| `hdr_histogram`| 1.42 [1.32, 1.52] % | 1.53 [1.42, 1.64] % | 1.66 [1.56, 1.75] % | **1.37 [1.09, 1.65] %** |
| `sharded_n`    | 1.71 [1.58, 1.84] % | 1.85 [1.70, 1.99] % | 2.19 [2.01, 2.36] % | 0.83 [0.60, 1.07] % |

*All strategies remain affordable in absolute terms: the maximum throughput overhead observed across the entire study is under 2.2%.*

### 6.2 Scaling Behavior (pp per Doubling of Client Concurrency)

| Axis | Fitted Slope | 95% CI | Runs with Positive Slope | Overhead ($c=100 \to c=3000$) |
|---|---:|---|---:|---|
| **CARD** | 0.0004 | [−0.026, 0.027] | 4 / 11 | 0.95 pp → 0.86 pp |
| **SYNC** | 0.037 | [−0.007, 0.080] | 10 / 11 | 0.56 pp → 0.60 pp |
| **DEFER**| 0.041 | [−0.009, 0.092] | 8 / 11 | 0.19 pp → 0.24 pp |
| **PAYLOAD** | 0.038 | [−0.024, 0.100] | 7 / 11 | 0.81 pp → 0.74 pp |

*Baseline server performance under saturation ($c=100 \to c=3000$): Throughput drops 10.5% (290,540 to 259,932 ops/s), while $p99$ increases from 0.76 ms to 78.85 ms.*

### 6.3 Overhead Rigidity (Tail-Concentration Index = $p99$ cost ÷ $p50$ cost)

| Strategy | $p50$ Cost | $p99$ Cost | Rigidity Index | 95% CI |
|---|---:|---:|---:|---|
| `sharded_n` | 2.19% | 0.83% | 0.38 | [0.28, 0.48] |
| `global_mutex` | 1.79% | 0.78% | 0.43 | [0.30, 0.55] |
| `sharded_2key` | 1.13% | 0.54% | 0.49 | [0.27, 0.71] |
| `thread_local` | 0.72% | 0.43% | 0.58 | [0.36, 0.81] |
| **`hdr_histogram`** | 1.66% | 1.37% | **0.82** | **[0.68, 0.96]** |

### 6.4 Follow-up Axis Separation Batch (8 Dedicated VMs)

| Quantity Measured | Estimate (pp) | 95% CI | Resolution |
|---|---:|---|---|
| **Pure Cardinality** (2 → 13,040 entries, identical work) | **0.652** | [0.280, 1.024] | Resolved ($p < 0.05$) |
| CARD Total (`sharded_n` − `sharded_2key`) | 0.642 | [0.316, 0.967] | Resolved |
| **Histogram Alone** (`hdr_histogram` − `thread_local_owned`) | **0.773** | [0.539, 1.006] | Resolved ($p < 0.05$) |
| PAYLOAD Total (`hdr_histogram` − `thread_local`) | 0.967 | [0.709, 1.226] | Resolved |
| Key Construction / FNV Hash (`sharded_bucketed`@1 − `sharded_2key`) | 0.855 | [0.393, 1.317] | Resolved |
| Owned-Key Allocation (`thread_local_owned` − `thread_local`) | 0.195 | [−0.017, 0.407] | Not Resolved ($p > 0.05$) |

### 6.5 Cardinality Threshold Sweep (`sharded_bucketed`)

| Active Entries ($C$) | Cost vs. `disabled` | Cost vs. $C=2$ Entries | Resolution |
|---:|---:|---|---|
| 2 | 1.707% | Reference | — |
| 20 | 1.805% | +0.097 [−0.107, 0.302] pp | Not resolved |
| 200 | 1.796% | +0.089 [−0.175, 0.352] pp | Not resolved |
| 2,000 | 2.014% | +0.306 [0.069, 0.543] pp | **Resolved** ($p < 0.05$) |
| 13,040 | 2.360% | +0.652 [0.280, 1.024] pp | **Resolved** ($p < 0.01$) |

### 6.6 Replication, Two-State Detection, and Crossover Decay
- **Two-State Bimodal Heuristic**: An automated detector checking for bimodal throughput distributions flagged **0 of 192 configurations** across both machine classes.
- **Crossover Decay**: Strategy rankings showed apparent "crossovers" in single runs (6 crossovers in 1 run). When pooling 10 runs, crossovers fell to 4; across 11 pooled runs, crossovers decayed to **0**. Apparent crossovers in small benchmarks were noise artifacts.

### 6.7 A Systematic Failure Mode: Open File Descriptors
During initial trials on Azure VMs with $c > 1,000$, throughput dropped to exactly 0 ops/sec with exit code 0. Cause: Fresh Linux instances had a default `RLIMIT_NOFILE` of 1024. The benchmark runner client encountered socket exhaustion and recorded 0 operations without crashing. The test runner script (`scripts/run_rmit_experiment.py`) now dynamically increases `RLIMIT_NOFILE` and asserts adequate system limits before initiating trials.

---

## 7. Threats to Validity and Limitations

1. **Loopback Co-location**: Client and server run on the same virtual machine over loopback (`127.0.0.1`). This design intentionally eliminates network switch variance to measure raw memory and synchronization costs, but it means client threads compete with server threads during CPU saturation.
2. **Core / Thread Counts**: Measured on 4 vCPU (`D4s_v6`) and 8 vCPU (`D8s_v6`) instances. While worker-thread scaling was validated between 4 and 8 cores, trends cannot be extrapolated to 64+ core NUMA topologies without further measurement.
3. **Hardware Homogeneity**: All 14 dedicated VMs used Intel Xeon Platinum 8573C processors. AMD EPYC, ARM Neoverse, or bare-metal execution profiles may exhibit different L1/L2 cache contention characteristics.
4. **Keyspace Storage Engine**: The primary paper results used the single-mutex keyspace backend (`Db`), representing the standard reference model. The sharded keyspace backend (`DbDashMap`) is included to verify that keyspace lock contention does not alter the relative ranking of metrics collectors (see [CONFOUND_CHECK.md](CONFOUND_CHECK.md)).

---

## 8. Reproducing the Paper

All datasets, analysis scripts, and figure generators are included in the repository.

### Step 1: Environment Setup
```bash
# Set up Python 3.10+ virtual environment
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

### Step 2: Fetch Experimental Data
The raw benchmark datasets (~7 MB compressed) are archived on Zenodo:
```bash
scripts/fetch_data.sh
```
This downloads and validates SHA256 checksums, populating `experiments/`.

### Step 3: Compile Server and Benchmark Harness
```bash
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
```

### Step 4: Run Experiments (Optional: Requires Cloud VMs)
To execute fresh RMIT trials (see [docs/rmit_experiment_protocol.md](rmit_experiment_protocol.md)):
```bash
# Main RMIT trial
python3 scripts/run_rmit_experiment.py --output-dir experiments/output
python3 scripts/analyze_rmit_results.py --input experiments/output/raw_data_rmit.csv

# Axis separation follow-up trial
python3 scripts/run_rmit_experiment.py \
  --strategies all --cardinalities 1,10,100,1000,10000 \
  --runs 15 --concurrency 100,1000,3000 --workloads mixed \
  --key-space 10000 --seed 2001 --output-dir experiments/output_followup
```

### Step 5: Regenerate Numbers and Figures
With the downloaded or generated data, run the analysis pipeline:
```bash
python3 scripts/analyze_design_axes.py          # Generates experiments/design_axes_analysis.json
python3 scripts/generate_design_axes_figures.py # Generates docs/figures/*.{png,pdf}
```
The analysis scripts output byte-identical tables, numbers, and figures as published in the manuscript.

---

## 9. Citation

If citing this research or utilizing the artifact, please use:

```bibtex
@article{rustredis_observability,
  title   = {Cardinality Before Contention: Decomposing Per-Command Observability Overhead on Concurrent Thread-Pool Key-Value Stores},
  author  = {RustRedis Research Team},
  journal = {Software: Practice and Experience (under submission)},
  year    = {2026},
  doi     = {10.5281/zenodo.23295261},
  url     = {https://doi.org/10.5281/zenodo.23295261}
}
```
See also [CITATION.cff](../CITATION.cff).
