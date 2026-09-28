# Observability Overhead Under Concurrency in an In-Memory Key-Value Store: An RMIT Measurement Study

*Draft. Experiments are complete: 4,320 RMIT runs on a Linux laptop and two
dedicated Azure VMs. Every table value and figure regenerates from committed
scripts (see the Appendix); citations and page ranges have been checked.
Known remaining work: (1) a live A/A run (§7); (2) author list and
affiliation; (3) reformatting for the target venue. The scope is a modest,
reproducible empirical measurement study rather than a new method; whether it
suffices for a given journal is a venue question for a supervisor or
co-author.*

## Abstract

RustRedis compares six command-metrics instrumentation strategies
(disabled, global mutex, sharded-2key, thread-local, HdrHistogram,
sharded-N) under concurrent load in a Rust Redis-compatible in-memory
server. Throughput on lightly controlled machines drifts over a run, so
comparing strategies in blocks would confound strategy with machine state
(Abedi and Brecht, 2017). We instead use Randomized Multiple Interleaved
Trials (RMIT; Abedi, Heard, and Brecht, 2015), which shuffles the order of
every (strategy, concurrency, workload) configuration within each repetition,
with per-run machine-state logging and a paired within-block comparison
against a no-metrics baseline. We run it on three machines — a 4-thread Linux
laptop and dedicated 4- and 8-vCPU Azure VMs — for 4,320 runs across 240
configurations, 25 to 3000 concurrent clients, and mixed, read-heavy, and
write-heavy workloads. No two-state throughput pattern appears in any
configuration (0 of 240 flagged), and mean relative 95% CI width is 0.9%-1.8%.
Every strategy costs 0.5%-2.1% throughput on average and at most 2.31% at any
tested concurrency level. ThreadLocal is cheapest and Sharded-2key second on
all three machines; the order of the remaining three strategies varies by
machine. A detectable-effect analysis (closed-form and injected-effect
simulation) puts the design's resolution at roughly 1-2% depending on the
dataset, so smaller differences between adjacent strategies are unresolved
rather than absent. We also document a failure that RMIT's own validity
checks do not catch: file-descriptor exhaustion that produced exactly
0 ops/sec with clean, well-formed output.

## Contributions

Randomized Multiple Interleaved Trials (RMIT) is Abedi, Heard, and Brecht's
technique (2015; Abedi and Brecht, 2017), and Laaber et al. (2019) already
combine RMIT-style sampling, bootstrap confidence intervals, A/A tests, and
simulation-based detectable-slowdown analysis for cloud microbenchmarks. Within
that frame, this paper contributes:

1. **An RMIT measurement of six instrumentation strategies on a live
   client-server workload (§4, §4b).** Abedi and Brecht (2017) replay traces
   collected by others, and Laaber et al. (2019) study single-method
   microbenchmarks and explicitly make no claims about load or stress tests.
   This paper applies RMIT to a network server under up to 3000 concurrent
   clients on 4-8 vCPUs, across three machines, and reports the overhead of six
   strategies (at most 2.31% per concurrency level) and which parts of their
   ranking replicate across machines (ThreadLocal first, Sharded-2key second).
2. **A "clean but wrong" failure that RMIT's own checks do not catch (§3).**
   File-descriptor exhaustion produced exactly `0 ops/sec` with `rc=0` and a
   validly-formed result in every affected configuration — the opposite of
   the noisy, bimodal instability that RMIT's bimodality detector and
   confidence-interval width are built to flag. We did not find an analogous
   failure reported in the RMIT papers above.
3. **A detectable-effect analysis for this specific comparison (§7).** By a
   closed-form paired-design estimate and by an injected-effect simulation on
   each dataset's own noise (following the injected-slowdown procedure of
   Laaber et al., 2019, adapted to within-machine paired blocks), the design
   detects overheads of roughly 1-2% at 80% power, with a 4-6% false-positive
   rate in the simulated null. The method is not new; it is reported so that
   "no significant difference" statements carry an explicit resolution floor.

## 1. Motivation and Related Work

Observability (per-command metrics/counters) is not free; the
implementation strategy for collecting it can dominate or disappear into
measurement noise depending on concurrency. This project asks two
questions that turn out to be almost independent of each other: (1) how
much does per-command metrics collection cost in a concurrent in-memory
key-value store, and which collection strategy costs least, and (2) can a
benchmark answer question (1) without a confound — such as drift in
machine state over the run — as large as the effect it is trying to measure
(§2).

**Benchmarking methodology.** Measured performance depends on setup
details experimenters rarely control or report (Mytkowicz et al., 2009),
so rigorous practice reports confidence intervals over repeated runs
(Georges et al., 2007; Kalibera and Jones, 2013) and randomizes factors
that should not matter but do — STABILIZER, for example, randomizes memory
layout (Curtsinger and Berger, 2013). Shared and cloud machines add
time-varying noise, both in compute (Maricq et al., 2018) and in the
network (Uta et al., 2020). Two designs address time-varying noise directly.
*Randomized Multiple Interleaved Trials (RMIT)* was proposed by Abedi,
Heard, and Brecht (2015) for WiFi experiments and shown by Abedi and Brecht
(2017) to be necessary in cloud environments: on replayed EC2 traces,
single-trial and multiple-consecutive-trial designs reported differences of
up to 37.8% between two identical systems, and even non-randomized
interleaving failed when the environment changed periodically. RMIT
interleaves trials of the alternatives under comparison in a fresh random
order each round, so drift lands on every alternative roughly equally.
*Duet benchmarking* (Bulej et al., 2020) instead runs the two artifacts
simultaneously so that shared noise cancels, reporting accuracy gains of
2.3x-82.4x; it does not suit a saturating client-server benchmark, where
two concurrent server instances would contend with each other, which is why
this paper interleaves sequentially. **The technique this paper applies in
§3 is RMIT; it is not new.** The closest prior work to this paper's overall
recipe is Laaber et al. (2019): RMIT-style "trial-based" sampling on the
same instances in randomized order, bootstrap confidence intervals, A/A
tests to measure false-positive rates, and a simulation-based
minimal-detectable-slowdown analysis (injecting slowdowns of 0.1%-1000%),
applied to 19 Java/Go microbenchmarks on three public clouds and a
bare-metal host. They find trial-based sampling markedly better than
comparing different instances — with five instances and five trials,
slowdowns in the 5%-10% range become detectable for most benchmarks — and
they explicitly make no claims about load or stress tests. For key-value and database
systems specifically, YCSB (Cooper et al., 2010) is the standard benchmark
suite, Reniers et al. (2017) survey under-reporting in NoSQL benchmark
practice, and Fruth et al. (2021) show that Java benchmark harnesses distort
measured tail latency — another instance of the harness, not the system,
dominating a result.

**Concurrency and observability costs.** Synchronization costs depend
heavily on hardware (David et al., 2013), which motivates comparing a
global lock, sharded counters, and per-thread accumulation. Sharded and
per-thread counting descend from low-contention counting networks (Aspnes et
al., 1994) and from the per-core "sloppy counters" used to remove kernel
counter contention (Boyd-Wickizer et al., 2010); replacing a global lock
with a concurrency-friendly structure is also what lifted Memcached's
throughput in MemC3 (Fan et al., 2013). Measurement tools can perturb the
contention they measure (Tallent et al., 2010), and production tracing
systems bound their own overhead by sampling because it is not negligible
(Sigelman et al., 2010) — both reasons to measure instrumentation overhead
directly rather than assume it away. HdrHistogram (Tene) supplies the
histogram recorder compared here.

**What is and is not new.** Not new: the RMIT technique (Abedi et al.,
2015; Abedi and Brecht, 2017), bootstrap confidence intervals over repeated
runs, A/A testing, and simulation-based detectable-effect analysis (Laaber
et al., 2019). What this paper offers, within that frame, is listed in the
Contributions section above: an RMIT measurement of six instrumentation
strategies on a live client-server workload, a "clean but wrong" failure
that RMIT's own checks do not catch, and an explicit detectable-effect
analysis for this specific comparison. Detailed per-paper notes are kept in
`docs/related_work_notes.md`.

## 2. Why Randomize Run Order

A benchmark that compares strategies must survive drift in machine state
over its wall-clock duration: thermal state, frequency scaling, background
activity, and co-tenants on shared hosts. If all repetitions of one
configuration run back to back before the next begins, a slow window that
lands during one configuration's block is statistically indistinguishable
from that configuration being genuinely slow (or, for a comparison against a
baseline, from a real overhead). The literature documents that this is not
hypothetical: on replayed EC2 traces, single-trial and
multiple-consecutive-trial designs reported differences of up to 37.8%
between two identical systems (Abedi and Brecht, 2017), and seemingly
innocuous, unrandomized setup differences silently produce wrong
conclusions across architectures and compilers (Mytkowicz et al., 2009).

This project therefore randomizes from the start. Every repetition (a
"block") runs all (strategy, concurrency, workload) configurations once in a
fresh random order, so any drifting condition is spread across strategies
rather than concentrated on one. The comparison the design makes valid is the
paired within-block ratio of each strategy's throughput to the `disabled`
baseline's in the same block: every strategy in a block saw the same
conditions, so a condition that slowed the block slows numerator and
denominator alike (§3, §4).

This paper does not include a fixed-order run on these machines, so it does
not measure how much run order would have mattered here; the case for
randomizing rests on the cited literature and on the standard argument above
(see §7).

## 3. The Design: RMIT

Randomized Multiple Interleaved Trials (RMIT) — the technique applied in
this section — is not new to this paper; it was proposed by Abedi, Heard,
and Brecht (2015) and shown to be necessary in cloud environments by
Abedi and Brecht (2017), as a fix for the class of problem §2 describes
(see §1 for the full comparison to those papers and to Laaber et al.
(2019), who apply the same technique with bootstrap confidence intervals to
cloud microbenchmarking). This project's specific application:

- Independent random permutation of the full (strategy x concurrency)
  matrix per repetition (`benchmarks/run_rmit_experiment.py`).
- Server restarted before every run (unavoidable once order is no longer
  strategy-grouped — every run is potentially a strategy switch).
- Machine-state snapshot logged immediately before every run
  (`benchmarks/system_state.py`): CPU frequency, thermal-zone temperature,
  memory/swap, load average, AC/battery status (fields degrade to `null`
  when unavailable, e.g. no thermal sensors or battery on a cloud VM).
- Three hardware targets, chosen to check that the overhead results are not
  specific to one machine:
  - **Laptop**: Intel i3-10110U (2C/4T), 8GB DDR4, Linux. 6 strategies x
    8 concurrency levels (25-500) x 15 repetitions = 720 runs. An active
    development session was running on this machine throughout (recorded
    as a caveat in `experiment_results_rmit/metadata_rmit.json`), so load
    average during runs was frequently 4-12 on a 4-thread CPU.
  - **Cloud VM (v1)**: Azure `Standard_D4s_v6` (4 vCPU, 16GB RAM, Central
    India), provisioned solely for the run and deleted immediately after.
    6 strategies x 8 concurrency levels (100-1000) x 30 repetitions =
    1440 runs. Nothing else ran on this
    machine — no thermal sensors, no battery, no competing session.
  - **Cloud VM (v2, advanced)**: Azure `Standard_D8s_v6` (8 vCPU, 32GB
    RAM, Central India), same provision-run-delete pattern. Extends the
    design to a third dimension — 6 strategies x 8 concurrency levels
    (100-3000) x 3 workload types (mixed, read-heavy, write-heavy) x 15
    repetitions = 2160 runs — to find where strategies actually diverge
    (higher concurrency) and whether read/write mix changes which
    strategy is cheapest (it doesn't; see §4b).

**A methodological pitfall worth recording**: the first attempt at the v2
run silently failed above roughly c=1000 — every run reported exactly
`0 ops/sec` with no error, rc=0, and a validly-formed JSON output. The
cause was the default open-file-descriptor soft limit (1024 on a fresh
Ubuntu VM) being far below the concurrency levels being tested; every
client thread's `connect()` failed, and the benchmark client counts
connection failures as errors internally rather than aborting, so the run
"succeeded" while measuring nothing. `run_rmit_experiment.py` now raises
its own `RLIMIT_NOFILE` toward the hard limit once at startup (inherited
by both the server and client subprocesses it spawns) and warns loudly if
the achieved limit still can't cover the requested concurrency. This is
recorded here because it is exactly the kind of failure RMIT's own
validation (§2's reasoning about drifting machine state) does not catch —
a systematic, order-independent failure produces suspiciously *clean*,
*consistent* zeros, not the noisy instability that randomizing run order
guards against. The timing/smoke test that caught it
(comparing a 1-repetition dry run's per-run throughput values by eye
before committing to the full 15-repetition run) is why it's worth always
inspecting raw per-run output before trusting an aggregate.

## 4. Results on the RMIT Machines

Both datasets and their analysis are in the repo:
`experiment_results_rmit/` (laptop) and `experiment_results_rmit_azure/`
(cloud VM), each with `raw_data_rmit.csv`, `rmit_analysis.json`,
`rmit_analysis_summary.csv`, and `metadata_rmit.json`.

**No two-state pattern on either machine.** `benchmarks/analyze_rmit_results.py`
flags a configuration as two-state when its throughput distribution
splits into two clusters at least 1.8x apart with each holding >=15% of
samples. Result: **0 of 48 configurations flagged on the laptop, 0 of 48
on the cloud VM.** The detector is a simple largest-gap heuristic (§5), so
this rules out large, well-separated state splits but not subtler
multimodality.

**Run-to-run variability is low.** Relative 95% bootstrap CI width on
throughput (width / median):

| | RMIT laptop (Linux) | RMIT cloud VM (Azure D4s_v6) |
|---|---:|---:|
| Mean across 48 configs | 0.0129 | 0.0091 |
| Max across 48 configs | 0.0386 | 0.0175 |

Variability this small (mean relative CI width 0.9%-1.8% across all three
datasets; §4b adds the third) is what makes the sub-2% overhead comparisons
below meaningful.

**Instrumentation overhead is small, consistent, and doesn't cross over.**
Paired within-repetition-block throughput ratios vs. the `disabled`
baseline (the RMIT-valid comparison — it controls for whatever
time-varying condition affected block N, since every strategy in block N
saw the same condition). For each (strategy, concurrency[, workload])
cell, overhead is 1 − the *median* paired-block throughput ratio (medians
are this project's primary point estimate throughout, per
`benchmarks/analyze_rmit_results.py`); "mean overhead" in every table below
is the arithmetic mean of those per-cell medians across cells:

| Strategy | Laptop mean overhead | Cloud VM mean overhead |
|---|---:|---:|
| thread_local | 0.99% | 0.56% |
| sharded_2key | 1.15% | 0.85% |
| global_mutex | 1.36% | 0.99% |
| sharded_n | 1.97% | 1.60% |
| hdr_histogram | 2.14% | 1.42% |

`thread_local` is cheapest on both machines, but the *most* expensive slot
is not identical: `hdr_histogram` is highest on the laptop (2.14%) while
`sharded_n` is highest on the cloud VM (1.60% vs. `hdr_histogram`'s 1.42%)
— the two swap places at the bottom of the ranking (see §4b for the full
three-dataset cross-check of which parts of this ranking replicate and
which don't). Overhead is small on both machines regardless (under 2.2%
everywhere), and the cloud VM's numbers are uniformly a bit lower than the
laptop's — consistent with the laptop dataset carrying some contention
from the development session sharing its 4 threads, rather than any
strategy behaving qualitatively differently between machines. Individual
per-block ratios ranged from 0.9573 to 0.9973 (laptop) and 0.9771 to
1.0051 (cloud VM) — the one ratio slightly above 1.0 is noise (a
strategy occasionally edging out `disabled` in a single block), not a
reversal of the overall ranking.

## 4b. Extending the Design: Workload Type and Higher Concurrency

The two datasets in §4 only exercise the 50/50 mixed workload and top out
at 1000 concurrent clients. Neither found where (or whether) strategies
actually diverge, and neither tested whether read/write mix interacts
with instrumentation cost. `experiment_results_rmit_advanced/` (Azure
`Standard_D8s_v6`) closes both gaps: 6 strategies x 8 concurrency levels
(100-3000) x 3 workloads (mixed, read-heavy, write-heavy) x 15
repetitions = 2160 runs.

**Still no two-state pattern, at 3x the concurrency ceiling and 3x the
workload coverage.** 0 of 144 configurations flagged two-state. Mean
relative 95% CI width: 0.0177 (max 0.0420) — slightly wider than the
smaller cloud-VM run (0.0091), consistent with 15 repetitions vs 30, and
still small.

**Overhead stays small and flat all the way to c=3000** — there is no
concurrency level where any strategy's cost jumps or the ranking
qualitatively changes:

| Concurrency | global_mutex | hdr_histogram | sharded_2key | sharded_n | thread_local |
|---:|---:|---:|---:|---:|---:|
| 100 | 1.65% | 2.10% | 1.09% | 1.77% | 0.03% |
| 250 | 1.48% | 1.86% | 1.57% | 1.70% | 0.57% |
| 500 | 2.06% | 1.15% | 0.61% | 2.08% | 1.00% |
| 750 | 1.30% | 1.44% | 0.21% | 2.12% | 0.27% |
| 1000 | 1.33% | 1.41% | 0.95% | 2.31% | 0.13% |
| 1500 | 1.62% | 0.76% | 0.87% | 1.92% | 0.69% |
| 2000 | 1.80% | 1.90% | 1.13% | 1.95% | 0.77% |
| 3000 | 1.91% | 1.59% | 1.06% | 2.00% | 0.68% |

(overhead relative to `disabled`, averaged over the 3 workloads at each
concurrency level.)

![Instrumentation overhead vs. concurrency, five strategies, Azure D8s_v6](../figures/fig1_overhead_vs_concurrency.png)

**Figure 1.** Mean overhead vs. `disabled` for each strategy across the full
100-3000 concurrency range (Azure D8s_v6 advanced dataset, averaged over
the three workload types). Generated directly from
`experiment_results_rmit_advanced/rmit_analysis.json` by
`benchmarks/generate_paper_figures.py` — the same source as the table
above, not a separate hand-plotted figure. The jaggedness from one
concurrency level to the next is itself informative: it is the visual
signature of every gap here being close to or below this dataset's
minimum detectable effect (§7), consistent with reading these as "small
and flat," not "trending," across the tested range.

`thread_local` never exceeds 1.0% at any concurrency
level tested; `sharded_n` never exceeds 2.31%. The server's own
throughput roughly saturates rather than collapses under load — disabled
baseline throughput drops from ~280-286k ops/sec at c=100 to ~231-239k at
c=3000 (about a 16-18% decline) while p99 latency rises from under 1ms to
83-87ms, a graceful saturation curve rather than a cliff.

**Workload type does not change which strategy is cheapest.** Averaging
overhead across all 8 concurrency levels for each workload:

| Strategy | Mixed | Read-heavy | Write-heavy |
|---|---:|---:|---:|
| thread_local | 0.44% | 0.46% | 0.64% |
| sharded_2key | 1.11% | 0.87% | 0.83% |
| hdr_histogram | 1.45% | 1.78% | 1.36% |
| global_mutex | 1.74% | 1.40% | 1.79% |
| sharded_n | 1.73% | 2.15% | 2.07% |

`thread_local` is cheapest and `sharded_2key` is second-cheapest in all
three workloads — that part of the ranking is exactly identical.
`sharded_n` is clearly the most expensive strategy in read-heavy (2.15%)
and write-heavy (2.07%); in mixed it is nominally 0.01 percentage points
behind `global_mutex` (1.73% vs. 1.74%), a gap so far below this design's
minimum detectable effect (§7) that the honest reading is "`global_mutex`
and `sharded_n` are tied for most expensive in the mixed workload, and
`sharded_n` is unambiguously most expensive in the other two." This is a
negative result worth stating plainly regardless: read/write mix was a
plausible place for instrumentation cost to interact with workload (e.g.
if a strategy's overhead were dominated by write-path lock contention, a
write-heavy workload might expose it more), and — for the two strategies
at the top of the ranking, and for `sharded_n` at the bottom — it does
not.

**The "crossovers" the analysis script flags are noise, not signal.**
`benchmarks/analyze_rmit_results.py`'s crossover detector (which strategy
has the highest median throughput at each concurrency level) reports 6
leader changes across the 3 workloads — e.g. `thread_local` and
`disabled` trade the lead multiple times in the mixed workload between
c=100 and c=1000. Given every strategy's overhead is within about 2.3% of
`disabled` at every concurrency level (previous table), a leader change
driven by sub-2%, sub-CI-width differences is exactly what pure
measurement noise looks like, not a genuine strategy-concurrency
interaction. We report the crossover count because the tooling now
supports detecting a real one, but the correct reading of this specific
dataset is "no meaningful crossover found" — overhead is flat and small
across the entire tested range.

**The top of the ranking is not noise, even though most adjacent gaps
are.** It's worth being precise about which parts of "no significant
difference" actually mean "we detected no difference" versus "any
difference here is too small for this design to see" (see the
minimum-detectable-effect numbers in §7). Ranking all six strategies by
mean overhead within each of the three independent datasets — laptop,
Azure D4s_v6, and Azure D8s_v6 (§4's table and this section's workload
table) — gives:

| Rank | Laptop | Azure D4s_v6 | Azure D8s_v6 (advanced) |
|---:|---|---|---|
| 1 (cheapest) | thread_local | thread_local | thread_local |
| 2 | sharded_2key | sharded_2key | sharded_2key |
| 3 | global_mutex | global_mutex | hdr_histogram |
| 4 | sharded_n | hdr_histogram | global_mutex |
| 5 (most expensive) | hdr_histogram | sharded_n | sharded_n |

![Mean overhead by strategy, grouped by dataset](../figures/fig2_cross_dataset_ranking.png)

**Figure 2.** The same ranking as a chart: `thread_local` (leftmost bar in
every group) and `sharded_2key` (second) hold their positions across all
three independently-provisioned machines; `global_mutex`, `hdr_histogram`,
and `sharded_n` reorder between groups. Generated by
`benchmarks/generate_paper_figures.py` from each dataset's own
`rmit_analysis.json`, using the same per-cell ratio-median statistic as
every table in §4 and §4b (verified to reproduce those tables' values
exactly before this script was trusted for the figure).

`thread_local` takes rank 1 and `sharded_2key` takes rank 2 in all three
independently-provisioned datasets — that part of the ranking is stable.
Ranks 3-4 reshuffle between datasets (`global_mutex` and `hdr_histogram`
trade places), and `sharded_n` is in the bottom two everywhere but is not
always dead last. No single adjacent-strategy gap in any one dataset
clears that dataset's minimum detectable effect, so none of this is a
"statistically significant" pairwise claim in any single run. But three
independently-provisioned machines agreeing on which strategy takes rank 1
and which takes rank 2 is not the behavior pure per-run noise would
produce — noise would put a different strategy on top in each dataset
about as often as not. We read the rank-1/rank-2 finding as a small,
consistent effect (plausibly because per-thread accumulation and a two-entry
concurrent map avoid synchronization or per-key bookkeeping cost that the
other three pay; we did not test the mechanism), distinct from both the crossover claims
above (which really are noise) and the ranks-3-through-5 reshuffling
(which is also most plausibly noise, since it doesn't replicate in a
fixed order across datasets). A crossover, and a rank-3-vs-4 swap, are
each single-dataset events with no cross-dataset replication behind them;
the rank-1/rank-2 finding replicates three times independently.

## 5. Machine-State Logging and What It Can Check

Every row of each `raw_data_rmit.csv` carries a machine-state snapshot taken
immediately before the run (`benchmarks/system_state.py`): mean and maximum
CPU frequency, governor, thermal-zone temperature, memory and swap
availability, 1/5/15-minute load average, and AC/battery status (fields are
`null` where the hardware lacks them, e.g. thermal sensors and batteries on
the Azure VMs). These logs allow post-hoc checks that a run was not
disturbed — for example, the laptop's development session kept its load
average at 4-12 on a 4-thread CPU throughout, which is recorded rather than
hidden (§7).

The two-state check in `benchmarks/analyze_rmit_results.py` is a deliberately
simple heuristic: split each configuration's throughputs at their largest gap
and flag it if the two clusters' medians are at least 1.8x apart and each
holds at least 15% of the samples. It flagged 0 of 240 configurations (0 of 48
on the laptop, 0 of 48 on Azure D4s_v6, 0 of 144 on Azure D8s_v6). Because it
is a largest-gap heuristic rather than a mixture-model fit, it could miss
subtler multimodality or state shifts smaller than 1.8x; the paired
within-block design does not depend on it, since it compares strategies
within blocks whatever the machine state was.
`benchmarks/hardware_hypothesis_check.sh` exists to test thermal-throttling,
governor-instability, and memory-pressure hypotheses directly if a
machine-state explanation for any future anomaly is needed.

## 6. Practical Implications

Two audiences can act on this work directly.

**For anyone choosing a metrics-collection strategy for a similar
concurrent server:** the overhead of per-command observability here is
small enough (at most 2.31% at every concurrency level tested, on every
workload and every machine) that "does instrumentation cost too much" is
not, by itself, a reason to leave it disabled in production for a system
in this class (single-process, in-memory, network-bound). Within that
small budget, the choice of *strategy* still matters in a stable way:
`thread_local` accumulation is the cheapest strategy on every machine we
tested and is the safe default when overhead must be minimized; if
per-command latency histograms (not just counters) are required,
`hdr_histogram` costs more but the extra cost (roughly 1.4-2.1
percentage points versus `disabled`, depending on machine) buys detailed
tail-latency visibility that flat counters cannot provide, so it is a
reasonable trade rather than a strategy to avoid. `sharded_n` — a
concurrent map keyed by the full logical (data) key, so it holds one entry
per distinct key (up to 10,000 in this workload) — showed no throughput
advantage over a plain `global_mutex` at the concurrency levels tested
(4-8 vCPUs, up to 3000 clients) and was in the bottom two on every machine;
the extra cost of per-key entries is not paying for itself here, though we
did not test why, and it would only be worth revisiting at core counts or
contention levels well beyond what this project tested (see §7's scope
limits). `sharded_2key`, by contrast, is keyed by command name (two entries,
GET and SET) and was second-cheapest on every machine.

**For anyone designing a similar concurrency benchmark:** the more
general and arguably more durable finding is methodological, not about
this server. A fixed-order design (finish every repetition of
configuration A, then move to configuration B) cannot distinguish "this
configuration is unstable" from "the machine happened to be in a slow
state while this configuration was running" — §2 cites evidence that this
is a real failure mode (differences of up to 37.8% between identical
systems). Randomizing configuration order per repetition (RMIT; Abedi and
Brecht, 2017) is a cheap fix — no new hardware, no new instrumentation, just
a different loop order — that makes a drifting environment land on every
configuration rather than on one, so that within-block paired comparisons
stay valid. (This paper did not run a fixed-order comparison on these
machines, so it does not quantify the benefit here; §7.) Any benchmark that
compares more than one configuration under conditions that can drift
over the run's wall-clock duration (thermal state, background load,
cache warmth, OS scheduling decisions) is exposed to the same
confound, independent of what is being measured — this is a general
argument for randomized-order benchmarking, not a claim specific to
metrics-collection strategies or to this codebase. §3's file-descriptor
pitfall is a second, separate, general lesson: a systematic
infrastructure failure can produce clean, low-variance, entirely
*consistent* wrong numbers, which is the opposite signature of the
noisy instability a randomized design is built to catch — no
benchmark design substitutes for inspecting a small dry run's raw
per-sample output by eye before trusting an aggregate.

## 7. Limitations

- **No fixed-order comparison.** Every dataset here uses RMIT, so the paper
  cannot say how much a blocked design would have distorted these machines'
  results; the argument for randomizing rests on the cited literature (§2).
  The claims supported are the RMIT-based overhead measurements themselves
  (§4, §4b) and the resolution floor of that design (below).
- No live A/A experiment (running the identical configuration twice) was
  performed; the false-positive rate reported in this section comes from a
  resampling-based simulated null, which validates the test's calibration
  under the observed noise but not the experimental procedure end to end.
  Laaber et al. (2019) run A/A tests as a standard check; a live A/A run
  (for example `disabled` against `disabled` in the same RMIT blocks) is the
  natural next step.
- §1's related-work discussion covers the closest benchmarking-methodology
  papers found, but is not a
  systematic literature review; a venue-specific submission should widen
  the search, e.g. into published YCSB-based throughput comparisons of
  Redis-class systems, which were not individually surveyed. Abedi and
  Brecht (2017) and Laaber et al. (2019) were read in full; the other cited
  papers were checked at abstract level (Bulej et al., 2020; Maricq et al.,
  2018; Uta et al., 2020) or by their well-known contribution.
- This project's benchmark client is custom-built rather than YCSB
  (Cooper et al., 2010), the de facto standard for KV-store benchmarking.
  The workload types (mixed/read-heavy/write-heavy, §3) are conceptually
  similar to YCSB's core workloads but are not YCSB itself, so this
  paper's absolute throughput numbers are not directly comparable to
  published YCSB results for other systems; the paired, within-block
  overhead ratios (§4, §4b) are unaffected by this since they compare
  this project's own strategies against its own `disabled` baseline using
  the same client.
- The laptop run had an active development session sharing its 4 threads
  throughout (see caveat in `experiment_results_rmit/metadata_rmit.json`);
  the cloud VM run does not have this confound and should be treated as
  the primary dataset where the two disagree, though in practice they
  agree closely (§4).
- The server process is restarted before every run, but OS-level state
  (page cache, TCP port reuse, kernel socket buffers) still carries over
  between runs on the same machine — full OS-level isolation (e.g., a
  fresh VM per run) was out of scope.
- Client and server always share the same machine (no `taskset` pinning
  or second-machine load generator was used for any RMIT dataset); at the
  highest concurrency levels tested (1000 clients / 4 vCPUs on the first
  cloud VM, 3000 clients / 8 vCPUs on the second) this is a genuine
  thread-oversubscription stress test, not a clean saturation curve
  measuring the server in isolation from its own load generator.
- Three hardware configurations were tested (one physical laptop, two
  cloud VMs, all x86-64, 4-8 vCPUs/threads). No claim is made about
  behavior on hardware with substantially different core counts, NUMA
  topology, or non-x86 architectures.
- The advanced (§4b) dataset uses 15 repetitions per configuration vs 30
  for the first cloud VM run, trading some statistical tightness (mean CI
  width 0.0177 vs 0.0091) for 3x the design coverage (workload type x
  wider concurrency range) within a comparable wall-clock budget.
- **What "no significant difference" can and can't rule out.** Two
  estimates of the smallest overhead this design can detect, per dataset,
  both computed from each dataset's `raw_data_rmit.csv`. (a) A closed-form
  paired-design estimate, `MDE ≈ (z_.975 + z_.80) × SD / sqrt(n)` (80% power,
  95% confidence), from the pooled SD of per-block strategy/`disabled`
  throughput ratios (`benchmarks/compute_mde.py`): **1.5% on the laptop**
  (SD 0.021, n=15 reps/cell), **0.9% on Azure D4s_v6** (SD 0.017, n=30), and
  **2.1% on Azure D8s_v6 / advanced** (SD 0.029, n=15). (b) An
  injected-effect simulation adapted from Laaber et al. (2019): for each
  cell, remove its own effect, inject an overhead x, resample n paired
  ratios, and test whether the 95% percentile-bootstrap CI of the median (the
  interval `analyze_rmit_results.py` reports) excludes 1
  (`benchmarks/simulate_mde.py`; 400 simulations x 400 bootstrap resamples per
  cell and x, seed 0). At x = 0 the test rejects in 4%-6% of simulations
  (nominal 5%). Pooled power reaches 80% at about **1.5%** (laptop; 0.71 at 1%
  and 0.86 at 1.5%), **1.0%-1.5%** (Azure D4s_v6; 0.80 at 1%, 0.96 at 1.5%),
  and **2.0%** (Azure D8s_v6; 0.50 at 1%, 0.81 at 2%). The two methods agree to
  within the simulation grid. Most of the
  adjacent-strategy gaps in §4 and §4b's tables (often 0.2-0.5
  percentage points) are below this floor in at least one dataset. This
  means "no significant crossover" and "adjacent strategies are
  statistically indistinguishable at a given concurrency level" should be
  read as *this design's power is exhausted at gaps below roughly 1-2%*,
  not as *no difference exists* — a true effect smaller than the relevant
  MDE could be present in any single dataset without this design being
  able to see it. The one claim in §4b that survives this caveat is that
  `thread_local` takes rank 1 (cheapest) and `sharded_2key` takes rank 2
  in all three independent datasets — replication across three
  separately-powered datasets is evidence even where no single dataset's
  pairwise CI excludes zero. Ranks 3-5 (`global_mutex`, `hdr_histogram`,
  `sharded_n`) reshuffle between datasets and are not claimed to be
  distinguishable from each other.

## 8. Conclusion

This paper measured the throughput cost of six per-command metrics
strategies in a Rust in-memory key-value server using Randomized Multiple
Interleaved Trials (Abedi, Heard, and Brecht, 2015; Abedi and Brecht, 2017)
on three machines — a Linux laptop and dedicated 4- and 8-vCPU Azure VMs —
over 4,320 runs and 240 configurations, from 25 to 3000 concurrent clients and
three workload mixes. No two-state throughput pattern appeared, and run-to-run
variability was low (mean relative 95% CI width 0.9%-1.8%). Every strategy
costs a small, flat overhead (0.5%-2.1% on average, at most 2.31% at any tested
concurrency), with `thread_local` cheapest and `sharded_2key` second on all three
machines. The design detects overheads of roughly 1-2% (§7), so finer
distinctions among the other three strategies are unresolved, not absent.
Two methodological points generalize beyond this server: randomizing
configuration order per repetition keeps a drifting environment from
concentrating on one configuration (§2, §6), and a systematic infrastructure
failure — here, file-descriptor exhaustion — can produce clean, uniform, wrong
results that RMIT's own checks do not flag, so a small dry run's raw output
should be inspected before trusting an aggregate (§3). Natural next steps are
a live A/A run, a second-machine load generator to remove client-server
co-location, and additional workloads and hardware.

## References

Author-year citations throughout this paper (e.g. "Abedi and Brecht,
2017") refer to the entries below, alphabetized by first author. Authors,
titles, venues, years, and page ranges were checked against publisher,
dblp, or USENIX/ACM records via search; entries without page ranges
(Boyd-Wickizer et al., Sigelman et al., Tene) are not paginated in the
form cited. Convert to the target venue's required style (numbered IEEE,
ACM reference format, etc.) at submission time — a mechanical
reference-manager step.

1. Abedi, A. and Brecht, T. (2017). Conducting Repeatable Experiments in
   Highly Variable Cloud Computing Environments. In *Proceedings of the
   8th ACM/SPEC International Conference on Performance Engineering
   (ICPE '17)*, 287-292.
2. Abedi, A., Heard, A., and Brecht, T. (2015). Conducting Repeatable
   Experiments and Fair Comparisons using 802.11n MIMO Networks. *ACM
   SIGOPS Operating Systems Review*, 49(1), 41-50.
3. Aspnes, J., Herlihy, M., and Shavit, N. (1994). Counting Networks.
   *Journal of the ACM*, 41(5), 1020-1048.
4. Boyd-Wickizer, S., Clements, A. T., Mao, Y., Pesterev, A., Kaashoek,
   M. F., Morris, R., and Zeldovich, N. (2010). An Analysis of Linux
   Scalability to Many Cores. In *Proceedings of the 9th USENIX Symposium
   on Operating Systems Design and Implementation (OSDI '10)*.
5. Bulej, L., Horky, V., Tuma, P., Farquet, F., and Prokopec, A. (2020).
   Duet Benchmarking: Improving Measurement Accuracy in the Cloud. In
   *Proceedings of the ACM/SPEC International Conference on Performance
   Engineering (ICPE '20)*, 100-107.
6. Cooper, B. F., Silberstein, A., Tam, E., Ramakrishnan, R., and Sears,
   R. (2010). Benchmarking Cloud Serving Systems with YCSB. In
   *Proceedings of the 1st ACM Symposium on Cloud Computing (SoCC '10)*,
   143-154.
7. Curtsinger, C. and Berger, E. D. (2013). STABILIZER: Statistically
   Sound Performance Evaluation. In *Proceedings of the 18th
   International Conference on Architectural Support for Programming
   Languages and Operating Systems (ASPLOS '13)*, 219-228.
8. David, T., Guerraoui, R., and Trigonakis, V. (2013). Everything You
   Always Wanted to Know About Synchronization but Were Afraid to Ask.
   In *Proceedings of the 24th ACM Symposium on Operating Systems
   Principles (SOSP '13)*, 33-48.
9. Fan, B., Andersen, D. G., and Kaminsky, M. (2013). MemC3: Compact and
   Concurrent MemCache with Dumber Caching and Smarter Hashing. In
   *Proceedings of the 10th USENIX Symposium on Networked Systems Design
   and Implementation (NSDI '13)*, 371-384.
10. Fruth, M., Scherzinger, S., Mauerer, W., and Ramsauer, R. (2021).
    Tell-Tale Tail Latencies: Pitfalls and Perils in Database
    Benchmarking. In *Performance Evaluation and Benchmarking (TPCTC
    2021)*, Lecture Notes in Computer Science 13169, 119-134.
11. Georges, A., Buytaert, D., and Eeckhout, L. (2007). Statistically
    Rigorous Java Performance Evaluation. In *Proceedings of the 22nd
    ACM SIGPLAN Conference on Object-Oriented Programming Systems,
    Languages, and Applications (OOPSLA '07)*, 57-76.
12. Kalibera, T. and Jones, R. E. (2013). Rigorous Benchmarking in
    Reasonable Time. In *Proceedings of the 2013 International Symposium
    on Memory Management (ISMM '13)*, 63-74.
13. Laaber, C., Scheuner, J., and Leitner, P. (2019). Software
    Microbenchmarking in the Cloud. How Bad is it Really? *Empirical
    Software Engineering*, 24(4), 2469-2508.
14. Maricq, A., Duplyakin, D., Jimenez, I., Maltzahn, C., Stutsman, R.,
    and Ricci, R. (2018). Taming Performance Variability. In *Proceedings
    of the 13th USENIX Symposium on Operating Systems Design and
    Implementation (OSDI '18)*, 409-425.
15. Mytkowicz, T., Diwan, A., Hauswirth, M., and Sweeney, P. F. (2009).
    Producing Wrong Data Without Doing Anything Obviously Wrong! In
    *Proceedings of the 14th International Conference on Architectural
    Support for Programming Languages and Operating Systems (ASPLOS
    '09)*, 265-276.
16. Reniers, V., Van Landuyt, D., Rafique, A., and Joosen, W. (2017). On
    the State of NoSQL Benchmarks. In *Companion of the 2017 ACM/SPEC
    International Conference on Performance Engineering (ICPE '17
    Companion)*, 107-112.
17. Sigelman, B. H., Barroso, L. A., Burrows, M., Stephenson, P.,
    Plakal, M., Beaver, D., Jaspan, S., and Shanbhag, C. (2010). Dapper,
    a Large-Scale Distributed Systems Tracing Infrastructure. Google
    Technical Report.
18. Tallent, N. R., Mellor-Crummey, J., and Porterfield, A. (2010).
    Analyzing Lock Contention in Multithreaded Applications. In
    *Proceedings of the 15th ACM SIGPLAN Symposium on Principles and
    Practice of Parallel Programming (PPoPP '10)*, 269-280.
19. Tene, G. HdrHistogram: A High Dynamic Range Histogram. Software,
    public domain. https://github.com/HdrHistogram/HdrHistogram
20. Uta, A., Custura, A., Duplyakin, D., Jimenez, I., Rellermeyer, J.,
    Maltzahn, C., Ricci, R., and Iosup, A. (2020). Is Big Data Performance
    Reproducible in Modern Cloud Networks? In *Proceedings of the 17th
    USENIX Symposium on Networked Systems Design and Implementation (NSDI
    '20)*, 513-527.

## Appendix: Reproducibility

```bash
# Laptop-class hardware
python3 benchmarks/run_rmit_experiment.py --runs 30 --output-dir experiment_results_rmit
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit/raw_data_rmit.csv

# Dedicated cloud VM, single workload (see docs/rmit_experiment_protocol.md for full az CLI steps)
RMIT_ENVIRONMENT_LABEL=<label> python3 benchmarks/run_rmit_experiment.py \
  --runs 30 --concurrency 100,200,300,400,500,600,700,1000 \
  --output-dir experiment_results_rmit_azure
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit_azure/raw_data_rmit.csv

# Dedicated cloud VM, advanced: 3 workloads x wider concurrency range
RMIT_ENVIRONMENT_LABEL=<label> python3 benchmarks/run_rmit_experiment.py \
  --runs 15 --concurrency 100,250,500,750,1000,1500,2000,3000 \
  --workloads mixed,read-heavy,write-heavy \
  --output-dir experiment_results_rmit_advanced
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit_advanced/raw_data_rmit.csv

# Figures 1 and 2 (§4b), generated from the three rmit_analysis.json files above
python3 benchmarks/generate_paper_figures.py

# Minimum detectable effect numbers (§7), from the three raw_data_rmit.csv files above
python3 benchmarks/compute_mde.py       # closed-form estimate
python3 benchmarks/simulate_mde.py      # injected-effect simulation (needs numpy; ~2 minutes)
```

Machine specs, commit hash, and full runtime config are written to each
run's `metadata_rmit.json`.
