# Observability Overhead Under Concurrency in an In-Memory Key-Value Store: A Corrected Benchmark Design

*Draft skeleton. Fill in results once the RMIT run (`docs/rmit_experiment_protocol.md`) completes;
replace bracketed placeholders. Target length: 4-6 pages.*

## Abstract

[One paragraph: RustRedis compares four-to-six command-metrics
instrumentation strategies (disabled, global mutex, sharded, thread-local,
HdrHistogram, sharded-N) under concurrent load. An earlier fixed-order
benchmark design produced results with an unexplained bimodal fast/slow
throughput pattern; we show this pattern is at least partly an artifact of
run ordering confounded with time-varying machine state, redesign the
experiment as randomized interleaved trials (RMIT) with machine-state
logging, and report corrected results.]

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

- `benchmarks/run_final_experiment_v12.py`: completes all N repetitions of
  one (strategy, concurrency) configuration before moving to the next
  (`experiment_results_v12/`).
- Evidence of the problem: [insert CV figures from
  `experiment_results_v12/aggregated_data.csv` and the
  `anomaly_investigation/` sharded-2key/c500 rerun — throughput CV as high
  as X%, bimodal distributions visible in
  `experiment_results_v12/graphs/throughput_distribution.png`].
- Why fixed order is unsound: any machine-state drift over the run's wall
  time (thermal, scheduler, memory) lands entirely inside whichever
  configuration happens to be running, making it indistinguishable from a
  genuine strategy or concurrency effect.

## 3. The Fixed Design: RMIT

- Randomized Multiple Interleaved Trials: independent random permutation
  of the full (strategy x concurrency) matrix per repetition
  (`benchmarks/run_rmit_experiment.py`).
- Server restarted before every run (unavoidable once order is no longer
  strategy-grouped).
- Machine-state snapshot logged with every run
  (`benchmarks/system_state.py`): CPU frequency, thermal-zone temperature,
  memory/swap, load average, AC/battery status.
- Hardware for this phase: Intel i3-10110U (2C/4T), 8GB DDR4, 512GB SSD,
  Linux — different from the Apple M2 used for v5/v12; concurrency levels
  and repetition counts were scaled down accordingly (see
  `docs/rmit_experiment_protocol.md`).

## 4. Corrected Results

[To fill in after the RMIT run completes and
`benchmarks/analyze_rmit_results.py` produces `rmit_analysis.json` /
`rmit_analysis_summary.csv`:]

- Per-configuration median throughput and p99 latency with bootstrap 95%
  CIs (table).
- Any configuration flagged as genuinely two-state (bimodal) by the
  analysis script, reported as two separate numbers rather than one
  misleading average — list which (strategy, concurrency) pairs, and the
  low/high state medians.
- Paired, within-repetition-block throughput ratios of each strategy
  against the `disabled` baseline — this is the RMIT-valid comparison,
  since every strategy in a given block experienced the same machine
  state.

## 5. Explaining the Machine States (Partial)

[Fill in from `benchmarks/hardware_hypothesis_check.sh` runs:]

- Hypothesis A (thermal throttling): [result — did frequency drop
  correlate with temperature crossing a threshold? Did a cooling
  pad/fan change the pattern?]
- Hypothesis B (frequency/governor instability): [result — did pinning
  the governor to `performance` or pinning server/client to separate
  cores with `taskset` change variance?]
- Hypothesis C (memory pressure): [result — did available memory or
  swap activity correlate with the slow state, particularly at higher
  concurrency levels on this 8GB machine?]
- [State whatever partial conclusion the evidence actually supports; it
  is fine to report "correlated with X, not fully explained" rather than
  claiming a definitive root cause.]

## 6. Limitations

- Single machine, single architecture (no cross-hardware replication in
  this phase).
- RMIT still shares a server process across many runs within a
  repetition's shuffled order in terms of OS-level state (page cache,
  TCP port reuse) even though the *metrics strategy* is freshly started
  per run — full OS-level isolation (e.g., a fresh VM per run) was out of
  scope for laptop-class hardware.
- Client and server share the same 4 hardware threads unless run with
  `taskset` pinning or a second physical machine as the load generator;
  results without that separation should be read as including some
  client/server resource contention.
- [Add any other caveats the actual RMIT run surfaces.]

## Appendix: Reproducibility

```bash
python3 benchmarks/run_rmit_experiment.py --runs 30 --output-dir experiment_results_rmit
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit/raw_data_rmit.csv
```

Machine specs, commit hash, and full runtime config are written to
`experiment_results_rmit/metadata_rmit.json` on every run.
