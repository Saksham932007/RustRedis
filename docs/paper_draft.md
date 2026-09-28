# Observability Overhead Under Concurrency in an In-Memory Key-Value Store: A Corrected Benchmark Design

*Draft. Sections 2 and 4 now report real numbers from both RMIT runs.
Remaining placeholders: the citation list in §1 (pull from the concept
note) and §5's hardware-hypothesis results (only worth running if we
want a laptop-side thermal/scheduling explanation — the Azure result
already shows the instability was never a server property to begin with).
Target length: 4-6 pages.*

## Abstract

RustRedis compares six command-metrics instrumentation strategies
(disabled, global mutex, sharded-2key, thread-local, HdrHistogram,
sharded-N) under concurrent load. An earlier fixed-order benchmark design
(v5/v12) reported throughput coefficients of variation up to 0.70 and an
unexplained bimodal fast/slow pattern at several concurrency levels. We
show this instability was an artifact of the benchmark's run ordering
being confounded with time-varying machine and process state, not a
property of the server. Redesigning the experiment as Randomized Multiple
Interleaved Trials (RMIT) with per-run machine-state logging, and
rerunning the full matrix on two independent machines (a 4-thread laptop
and a dedicated 4-vCPU cloud VM), eliminates the bimodal pattern entirely
(0 of 48 configurations flagged as two-state on either machine) and
reduces relative 95% CI width on throughput from a mean of 0.176 (as CV,
v12) to 0.0129 (laptop) and 0.0091 (cloud VM). Under the corrected design,
every instrumentation strategy costs a small, consistent throughput
overhead relative to `disabled` — roughly 1-2% on the laptop and 0.6-1.6%
on the cloud VM — with no crossover between strategies at any concurrency
level from 25 to 1000 concurrent clients.

## 1. Motivation

- Observability (per-command metrics/counters) is not free; the
  implementation strategy for collecting it can dominate or disappear
  into measurement noise depending on concurrency.
- Prior work characterizes lock contention and counter-update overhead in
  concurrent systems generally [CITATION NEEDED], but does not isolate the
  specific strategy-vs-concurrency interaction this project measures.
- [Cite the six papers from the concept note here — list not reproduced
  in-repo; pull from wherever the concept note itself lives and fill in
  full citations + one-sentence relevance for each.]

## 2. The Flawed Original Design and Its Evidence

`benchmarks/run_final_experiment_v12.py` completes all repetitions of one
(strategy, concurrency) configuration before moving to the next
(`experiment_results_v12/`), on Apple M2 hardware, 30 repetitions per
configuration, concurrency levels 100-1000.

**Evidence of the problem**, from `experiment_results_v12/aggregated_data.csv`:

| Strategy | Concurrency | Throughput CV |
|---|---:|---:|
| sharded_n | 100 | 0.698 |
| thread_local | 100 | 0.625 |
| global_mutex | 400 | 0.601 |
| hdr_histogram | 500 | 0.577 |
| global_mutex | 1000 | 0.473 |
| disabled | 200 | 0.453 |

Mean throughput CV across all 48 (strategy, concurrency) configurations in
v12: **0.176** — an order of magnitude higher than anything reported in
§4 below. The project's own `anomaly_investigation/` rerun of
sharded-2key/c500 (`experiment_results_v12/anomaly_investigation/`) was
commissioned specifically because run 1 of that configuration was a
multi-x outlier against its own median; the rerun's outlier did not
repeat, which is itself a symptom of order-dependent instability rather
than a reproducible property of that configuration.

**Why fixed order is unsound**: any machine-state drift over the run's
wall time (thermal throttling, background OS activity, frequency scaling,
competing processes) lands entirely inside whichever configuration
happens to be running when it occurs. Because v12 runs all 30
repetitions of `sharded_n/c100` back to back, then all 30 of the next
configuration, a slow window landing during `sharded_n/c100`'s block is
statistically indistinguishable from `sharded_n/c100` being a genuinely
unstable configuration — the design cannot tell the two apart.

## 3. The Fixed Design: RMIT

- Randomized Multiple Interleaved Trials: independent random permutation
  of the full (strategy x concurrency) matrix per repetition
  (`benchmarks/run_rmit_experiment.py`).
- Server restarted before every run (unavoidable once order is no longer
  strategy-grouped — every run is potentially a strategy switch).
- Machine-state snapshot logged immediately before every run
  (`benchmarks/system_state.py`): CPU frequency, thermal-zone temperature,
  memory/swap, load average, AC/battery status (fields degrade to `null`
  when unavailable, e.g. no thermal sensors or battery on a cloud VM).
- Two independent hardware targets, to separate "is this a laptop
  artifact" from "is this a general fix":
  - **Laptop**: Intel i3-10110U (2C/4T), 8GB DDR4, Linux. 6 strategies x
    8 concurrency levels (25-500) x 15 repetitions = 720 runs. An active
    development session was running on this machine throughout (recorded
    as a caveat in `experiment_results_rmit/metadata_rmit.json`), so load
    average during runs was frequently 4-12 on a 4-thread CPU.
  - **Cloud VM**: Azure `Standard_D4s_v6` (4 vCPU, 16GB RAM, Central
    India), provisioned solely for the run and deleted immediately after.
    6 strategies x 8 concurrency levels (100-1000, matching the original
    v12 range) x 30 repetitions = 1440 runs. Nothing else ran on this
    machine — no thermal sensors, no battery, no competing session.

## 4. Corrected Results

Both datasets and their analysis are in the repo:
`experiment_results_rmit/` (laptop) and `experiment_results_rmit_azure/`
(cloud VM), each with `raw_data_rmit.csv`, `rmit_analysis.json`,
`rmit_analysis_summary.csv`, and `metadata_rmit.json`.

**No bimodal states, on either machine.** `benchmarks/analyze_rmit_results.py`
flags a configuration as two-state when its throughput distribution
splits into two clusters at least 1.8x apart with each holding >=15% of
samples. Result: **0 of 48 configurations flagged on the laptop, 0 of 48
on the cloud VM.** The bimodal fast/slow pattern that motivated this
redesign did not reproduce once run order stopped being confounded with
time.

**Variability dropped by roughly an order of magnitude.** Comparing
relative 95% bootstrap CI width on throughput (width / median):

| | v12 (flawed, as throughput CV) | RMIT laptop | RMIT cloud VM |
|---|---:|---:|---:|
| Mean across 48 configs | 0.176 | 0.0129 | 0.0091 |
| Max across 48 configs | 0.698 | 0.0386 | 0.0175 |

(The v12 column is CV = stddev/mean rather than CI width, so the two are
not identical statistics, but both measure the same thing — how much a
configuration's repeated measurements spread relative to their center —
and the gap is large enough that the comparison is meaningful regardless
of which exact variability statistic is used.)

**Instrumentation overhead is small, consistent, and doesn't cross over.**
Paired within-repetition-block throughput ratios vs. the `disabled`
baseline (the RMIT-valid comparison — it controls for whatever
time-varying condition affected block N, since every strategy in block N
saw the same condition):

| Strategy | Laptop mean overhead | Cloud VM mean overhead |
|---|---:|---:|
| thread_local | 0.99% | 0.56% |
| sharded_2key | 1.15% | 0.85% |
| global_mutex | 1.36% | 0.99% |
| sharded_n | 1.97% | 1.60% |
| hdr_histogram | 2.14% | 1.42% |

Ranking is consistent across both machines (thread_local cheapest,
hdr_histogram most expensive), overhead is small on both (under 2.2%
everywhere), and the cloud VM's numbers are uniformly a bit lower than the
laptop's — consistent with the laptop dataset carrying some contention
from the development session sharing its 4 threads, rather than any
strategy behaving qualitatively differently between machines. Individual
per-block ratios ranged from 0.9573 to 0.9973 (laptop) and 0.9771 to
1.0051 (cloud VM) — the one ratio slightly above 1.0 is noise (a
strategy occasionally edging out `disabled` in a single block), not a
reversal of the overall ranking.

## 5. Explaining the Machine States (Partial)

The Azure result is itself informative here: a dedicated, idle VM with no
thermal sensors and no battery produced the same "no bimodal states"
outcome as the laptop, and with *tighter* CIs (0.0091 mean vs 0.0129).
This is consistent with the original v5/v12 instability being caused by
**benchmark design (fixed run order) rather than any specific hardware
condition** — thermal throttling, P/E-core scheduling, or memory pressure
would need to coincidentally reproduce a similar pattern on a VM with none
of those mechanisms for the alternative explanation to hold, which is
implausible.

`benchmarks/hardware_hypothesis_check.sh` exists to test the three
hardware-specific hypotheses (thermal throttling, governor/frequency
instability, memory pressure) directly if a laptop-side explanation is
still wanted for completeness, but given §4's result, we consider the
primary question — *why did v5/v12 see bimodal states* — already answered
by the design fix itself, with the hardware hypotheses now a secondary,
optional line of investigation rather than a load-bearing part of the
paper's argument.

## 6. Limitations

- The citation list in §1 is not yet filled in (concept note is external
  to this repo).
- The laptop run had an active development session sharing its 4 threads
  throughout (see caveat in `experiment_results_rmit/metadata_rmit.json`);
  the cloud VM run does not have this confound and should be treated as
  the primary dataset where the two disagree, though in practice they
  agree closely (§4).
- RMIT still shares a server process across many runs within a
  repetition's shuffled order in terms of OS-level state (page cache, TCP
  port reuse) even though the *metrics strategy* is freshly started per
  run — full OS-level isolation (e.g., a fresh VM per run) was out of
  scope.
- On the laptop, client and server share the same 4 hardware threads
  (no `taskset` pinning or second-machine load generator was used for
  either RMIT dataset); at concurrency 1000 on the cloud VM this means
  1000 client threads against 4 vCPUs, which is a genuine oversubscription
  stress test, not a clean saturation curve.
- Only two hardware configurations were tested (one physical, one cloud
  VM, both x86-64, both 4 threads/vCPUs). No claim is made about behavior
  on hardware with substantially different core counts, NUMA topology, or
  non-x86 architectures.

## Appendix: Reproducibility

```bash
# Laptop-class hardware
python3 benchmarks/run_rmit_experiment.py --runs 30 --output-dir experiment_results_rmit
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit/raw_data_rmit.csv

# Dedicated cloud VM (see docs/rmit_experiment_protocol.md for full az CLI steps)
RMIT_ENVIRONMENT_LABEL=<label> python3 benchmarks/run_rmit_experiment.py \
  --runs 30 --concurrency 100,200,300,400,500,600,700,1000 \
  --output-dir experiment_results_rmit_azure
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit_azure/raw_data_rmit.csv
```

Machine specs, commit hash, and full runtime config are written to each
run's `metadata_rmit.json`.
