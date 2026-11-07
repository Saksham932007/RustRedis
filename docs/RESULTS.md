# Results and experimental design

Detailed findings from the RustRedis observability-cost study. The README has the
short version; the manuscript ([final_paper_v27.md](final_paper_v27.md)) is the
authoritative source for every number below.

## TL;DR

Six interchangeable per-command metrics collectors are chosen so that adjacent pairs
differ along **exactly one design axis** — synchronization discipline, metric key
cardinality, aggregation locality, per-sample payload — which turns a strategy ranking
into a **factorial decomposition**. (Two further collectors were added for the
axis-separation experiment, eight in the tree.) Measurement uses Randomized Multiple Interleaved
Trials (RMIT; Abedi, Heard & Brecht 2015, shown necessary for clouds by
[Abedi & Brecht, ICPE 2017](final_paper_v27.md#2-background-and-related-work) — not
invented here), replicated across independently provisioned Azure VMs with all intervals
**clustered on runs, not blocks**: **29,520 benchmark runs** (25,200 main design +
4,320 axis-separation) over **fourteen dedicated cloud VMs**, 100–3000 concurrent
clients, three read/write mixes.

Four findings:

1. **Cardinality beats contention.** Moving the metric key from a 2-entry command-name
   label to a ~10,000-entry data-key label costs **0.87 pp** of throughput — **1.66×
   more** than the 0.52 pp that replacing a global mutex with a 64-way sharded map
   *saves* (gap 0.34 pp, 95% CI [0.19, 0.50], 10/11 runs, sign p = 0.012). A sharded
   "scalable" map with a high-cardinality label is measurably **slower** than one global
   lock with a low-cardinality one. 4.5× at 4 workers and 1.66× at 8 — or 4.51× and
   1.58× once the two worker counts are matched on workload and client load, which is
   the comparison to use when pairing them (§5.2).
2. **The axes obey different scaling laws.** Over a **30× client-concurrency range no
   axis moves** (largest slope 0.041 pp/doubling, CI spans zero) even though the server
   is genuinely saturated (p99 0.76 ms → 78.85 ms). The synchronization axis tracks
   *worker threads* instead (**2.6×** from 4 to 8 workers). Turning up client count
   cannot expose telemetry contention on a thread-pool server.
3. **Overhead has rigidity.** Counter costs are *elastic* — only 38–58% of their median
   cost survives into p99, absorbed by queueing slack. Histogram payload cost is
   *rigid*: statistically flat at 0.89–0.94 pp across throughput, mean, p50 and p99.
   This hides a sign reversal: HdrHistogram vs. GlobalMutex is **unresolved on
   throughput** (p = 0.23, even at 11× replication) yet resolves **in opposite
   directions** at p50 (cheaper, p = 0.001) and p99 (costlier, p = 0.001).
4. **The cardinality effect is real, and bounded.** A purpose-built collector whose
   per-operation work is identical at every cardinality confirms entry count accounts
   for essentially all of the cardinality axis (and the histogram for ~80% of payload) —
   but the effect is **indistinguishable from zero below ~10³ entries**.

Everything is affordable in absolute terms: no strategy costs more than **1.71 pp** of
throughput (2.19 pp at p50; the paper quotes the overhead ceiling as ~2.2%). The question
is never *whether* to instrument, but *how*.

Full write-up: **[docs/final_paper_v27.md](final_paper_v27.md)** — *"Cardinality
Before Contention"*, prepared for submission to *Software: Practice and Experience*.

## The collectors as a factorial design

| Strategy | Synchronization | Cardinality | Aggregation | Payload |
|---|---|---:|---|---|
| `disabled` | — | 0 | — | — |
| `global_mutex` | one global `Mutex<HashMap>` | 2 | eager, shared | counter |
| `sharded_2key` | `DashMap`, 64 shards | 2 | eager, shared | counter |
| `sharded_n` | `DashMap`, 64 shards | ~10,000 (data key) | eager, shared | counter |
| `thread_local` | thread-local map + periodic flush | 2 | deferred, private | counter |
| `hdr_histogram` | thread-local map + periodic flush | 2 | deferred, private | counter + HDR histogram |
| `sharded_bucketed` † | `DashMap`, 64 shards | **configurable `C`** | eager, shared | counter |
| `thread_local_owned` † | thread-local map + periodic flush | 2 | deferred, private | counter, owned key |

† Added for the axis-separation experiment. `sharded_bucketed` keys on
`"<CMD>#<fnv1a(key) mod C>"` so per-operation work is **identical at every `C`**
(`RUSTREDIS_METRICS_CARDINALITY`); sweeping `C` varies live entry count and nothing else.
`thread_local_owned` is a line-for-line twin of `hdr_histogram` with the histogram
replaced by a plain counter.

Timing (`Instant::now()`/`elapsed()`) and the key-hint lookup run identically for **all**
strategies including `disabled`, so those costs cancel in the paired comparison. Only the
body of `record()` varies.

The four contrasts, each a difference against a shared reference (so arbitrary strategy
comparisons are recoverable by arithmetic):

| Axis | Contrast | D8s_v6 (8 workers) | D4s_v6 (4 workers) |
|---|---|---|---|
| **CARD** | `sharded_n` − `sharded_2key` | **0.87 [0.73, 1.00]** | **0.79 [0.62, 0.96]** |
| **PAYLOAD** | `hdr_histogram` − `thread_local` | 0.89 [0.80, 0.97] | 1.00 [0.71, 1.28] |
| **SYNC** | `global_mutex` − `sharded_2key` | **0.52 [0.42, 0.63]** | **0.18 [−0.02, 0.38]** |
| **DEFER** | `sharded_2key` − `thread_local` | 0.31 [0.21, 0.41] | 0.26 [−0.00, 0.51] |

pp of throughput vs. `disabled`; all four hold in 11/11 runs. Since both are differences
against `sharded_2key`, **CARD − SYNC is algebraically `sharded_n` − `global_mutex`** —
which is why finding 1 has a direct physical reading. SYNC is contaminated *against* our
own hypothesis (`global_mutex` instruments its own lock wait into a contended atomic), so
the measured gap is a conservative one.

## Datasets

| Dataset | Hardware | Design | Runs |
|---|---|---|---:|
| [`azure_d4s_v6`](../experiments/azure_d4s_v6) | `Standard_D4s_v6`, 4 vCPU / 16 GB, Central India | 6 strategies × 8 concurrency (100–1000) × 30 blocks, mixed | 1,440 |
| [`azure_d8s_v6/pooled`](../experiments/azure_d8s_v6/pooled) | `Standard_D8s_v6`, 8 vCPU / 32 GB, **5 VMs / 3 regions** | 6 strategies × 8 concurrency (100–3000) × 3 workloads × 15 blocks, **× 11 independent runs** | 23,760 |
| [`azure_d8s_v6_followup/pooled`](../experiments/azure_d8s_v6_followup/pooled) | `Standard_D8s_v6`, **8 VMs / 3 regions** | 12 configs (incl. 5-point cardinality sweep) × 3 concurrency × 15 blocks, **× 8 independent runs** | 4,320 |

Every VM was provisioned solely for the benchmark and deleted afterwards; all report the
same Intel Xeon Platinum 8573C, so the pools vary in instance, region and date but not
microarchitecture. `scripts/combine_iterations.py` pools runs, offsetting each run's
`block_id` so paired comparisons never mix blocks across VMs. All overhead figures are
**paired within-block ratios** against the `disabled` baseline in the same block — the
comparison RMIT makes valid.

**Inference clusters on runs.** The pooled 165 blocks per cell are 11 independent runs ×
15 correlated within-run blocks; treating them as independent overstates precision by
~20%. Intervals collapse each run to one mean and form a *t*-interval over those numbers,
with directional claims cross-checked by an exact sign test.

## Results

### Cost by budget metric (pooled D8s_v6, % vs. `disabled`, 95% CI clustered on 11 runs)

| Strategy | Throughput | Mean latency | p50 | p99 |
|---|---|---|---|---|
| ThreadLocal | 0.53 [0.42, 0.65] | 0.62 [0.50, 0.74] | 0.72 [0.58, 0.85] | 0.43 [0.22, 0.64] |
| Sharded-2key | 0.84 [0.77, 0.92] | 0.94 [0.86, 1.01] | 1.13 [1.03, 1.23] | 0.54 [0.31, 0.77] |
| GlobalMutex | 1.37 [1.27, 1.46] | 1.49 [1.39, 1.58] | 1.79 [1.71, 1.88] | 0.78 [0.54, 1.02] |
| HdrHistogram | 1.42 [1.32, 1.52] | 1.53 [1.42, 1.64] | 1.66 [1.56, 1.75] | **1.37 [1.09, 1.65]** |
| Sharded-N | 1.71 [1.58, 1.84] | 1.85 [1.70, 1.99] | 2.19 [2.01, 2.36] | 0.83 [0.60, 1.07] |

The *ranking itself depends on the metric*: HdrHistogram is rank 4 on throughput, rank 3
at p50, rank 5 at p99. ThreadLocal is cheapest under every metric.

### Scaling (axes, pp per doubling of clients, fitted within run)

| Axis | Slope | 95% CI | Runs positive | c=100 → c=3000 |
|---|---:|---|---:|---|
| CARD | 0.0004 | [−0.026, 0.027] | 4/11 | 0.95 → 0.86 pp |
| SYNC | 0.037 | [−0.007, 0.080] | 10/11 | 0.56 → 0.60 pp |
| DEFER | 0.041 | [−0.009, 0.092] | 8/11 | 0.19 → 0.24 pp |
| PAYLOAD | 0.038 | [−0.024, 0.100] | 7/11 | 0.81 → 0.74 pp |

Meanwhile the `disabled` baseline saturates for real: 290,540 ops/s @ p99 0.76 ms at 100
clients → 259,932 ops/s @ p99 78.85 ms at 3000 (p99 ×104, throughput −10.5%). Graceful
saturation, no cliff — and telemetry cost simply does not participate. At matched client
load, 4 → 8 workers moves **SYNC 2.60×** and DEFER 1.39×, while PAYLOAD (0.94×) and CARD
(0.92×) do not move. Because SYNC grows with parallelism and CARD does not, their ratio
must close — extrapolating the matched SYNC trend puts a crossover at about 11 workers
(about 12 if SYNC is instead fitted linearly in worker count), but that is a hypothesis
from two worker-count points, not a fitted law; present claims are bounded to 4–8 cores.

### Rigidity (tail-concentration index = p99 cost ÷ p50 cost)

| Strategy | p50 | p99 | Index | 95% CI |
|---|---:|---:|---:|---|
| Sharded-N | 2.19% | 0.83% | 0.38 | [0.28, 0.48] |
| GlobalMutex | 1.79% | 0.78% | 0.43 | [0.30, 0.55] |
| Sharded-2key | 1.13% | 0.54% | 0.49 | [0.27, 0.71] |
| ThreadLocal | 0.72% | 0.43% | 0.58 | [0.36, 0.81] |
| **HdrHistogram** | 1.66% | 1.37% | **0.82** | [0.68, 0.96] |

Per axis, SYNC/CARD/DEFER shrink 60–75% from p50 to p99; **PAYLOAD is flat (0.89 → 0.94
pp)**. Proposed mechanism — a periodic O(buckets) HDR merge under a global lock *creates*
tail events rather than competing with queueing — is labelled interpretation, not
measurement: the merge was not instrumented directly.

### Axis separation (8-VM follow-up, pp, 95% CI clustered on 8 runs)

| Quantity | Estimate | 95% CI | |
|---|---:|---|---|
| **pure cardinality** (2 → 13,040 entries, per-op work identical) | **0.652** | [0.280, 1.024] | resolved |
| CARD total (`sharded_n` − `sharded_2key`) | 0.642 | [0.316, 0.967] | resolved |
| **histogram alone** (`hdr_histogram` − `thread_local_owned`) | **0.773** | [0.539, 1.006] | resolved |
| PAYLOAD total (`hdr_histogram` − `thread_local`) | 0.967 | [0.709, 1.226] | resolved |
| key construction, FNV + `format!` (`sharded_bucketed`@1 − `sharded_2key`) | 0.855 | [0.393, 1.317] | resolved |
| owned-key allocation, TLS path (`thread_local_owned` − `thread_local`) | 0.195 | [−0.017, 0.407] | **not** resolved |

The anchors reproduce across batches (SYNC 0.527 vs. 0.522, DEFER 0.290 vs. 0.309,
PAYLOAD 0.967 vs. 0.885), so the batches are comparable. **Cardinality is cardinality,
not allocation**, and payload is ~80% histogram.

**The cardinality curve — cost is not linear in entries, it appears above ~10³:**

| Data entries | Cost vs. `disabled` | vs. 2 entries | |
|---:|---:|---|---|
| 2 | 1.707% | (reference) | |
| 20 | 1.805% | +0.097 [−0.107, 0.302] | not resolved |
| 200 | 1.796% | +0.089 [−0.175, 0.352] | not resolved |
| 2,000 | 2.014% | +0.306 [0.069, 0.543] | resolved |
| 13,040 | 2.360% | +0.652 [0.280, 1.024] | resolved |

Teams already holding label cardinality to tens or low hundreds buy no throughput by
shrinking further (the *backend* cost of cardinality is a separate argument, unaffected).

**What this batch does *not* establish:** the strong form of finding 1. Pure cardinality
minus SYNC is +0.125 pp, CI [−0.358, 0.607], 4/8 runs — unresolved. The paper claims the
*package* (CARD) against SYNC on the 11-run batch, not the stronger ordering.

**An unplanned demonstration of why pairing matters:** one of the eight VMs (`run_06`)
ran ~30% below its peers on every configuration. Excluding it moves every contrast by
< 0.05 pp, because the paired within-block ratio cancels a level shift that hits
numerator and denominator alike. A design comparing absolute throughputs across
instances would have been badly distorted.

### Replication, two-state detection, crossovers

- **No two-state throughput pattern:** the RMIT detector
  (`scripts/analyze_rmit_results.py`) flags **0 of 192** configurations (0/48 on D4s_v6,
  0/144 on pooled D8s_v6). It is a largest-gap heuristic, so this rules out large,
  well-separated state splits, not subtler multimodality.
- **Crossovers vanish with data:** the leader-change count fell **6 → 4 → 0** as 1 → 10
  → 11 runs were pooled. Monotonic decay, not stabilization, is itself evidence the
  earlier "crossovers" were sub-2% noise swaps.
- **Pooling does not tighten everything.** Paired-ratio comparisons tighten ~3× (the
  detectable-effect floor drops from ~2.1% for one run to ~0.6–0.75% pooled, computed
  three agreeing ways). But raw relative CI width on throughput *widened* (mean 0.0091
  on D4s_v6, 0.0194 pooled), because it reflects genuine run-to-run variance that more
  within-run samples cannot shrink — and HdrHistogram vs. GlobalMutex got statistically
  *weaker* with the 11th run (p ≈ 0.11 → 0.23). Finding 3 explains why: that comparison
  is sign-ambiguous, not underpowered.

### A failure mode worth recording

The first D8s_v6 attempt reported exactly `0 ops/sec` above ~1000 clients — return code
0, well-formed output, no error. Cause: the default open-FD limit (1024) on a fresh VM
was far below the tested concurrency, so every client connection failed and the client
counted failures internally. `scripts/run_rmit_experiment.py` now raises `RLIMIT_NOFILE`
and warns when the limit cannot cover the requested concurrency. This is the *inverse* of
what RMIT defends against: randomization protects against noisy drifting confounds, while
this was a systematic failure producing suspiciously clean numbers. No design substitutes
for reading a small dry run's raw output.

### Caveats

- One VM family, one x86-64 OS image: replicated across VM size, instance, region and
  workload, but not across clouds or hardware families.
- Client and server share the machine; at the highest concurrency this is a genuine
  thread-oversubscription stress test, not an isolated server measurement.
- Only 4 and 8 vCPU/worker counts tested — no claim about other architectures or core
  counts, including the extrapolated SYNC/CARD crossover.
- A logging bug recorded a placeholder processor string in the D4s_v6 run's metadata, so
  while all eleven D8s_v6 runs verifiably share one CPU model (Intel Xeon Platinum
  8573C), the artifact cannot confirm the 4-vCPU VM used the same microarchitecture.
- The ~10³-entry threshold of finding 4 is measured with one instrument on one server and
  may be a property of this DashMap configuration rather than of the class.
- The HDR-merge mechanism for rigidity is interpretation, not direct measurement.
