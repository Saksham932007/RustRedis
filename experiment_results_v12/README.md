# experiment_results_v12

This directory contains reproducible v12 experiment artifacts.

## Contents

- `raw_data.csv`: per-run data (`strategy, concurrency, run_id, throughput, p99`, plus additional diagnostics)
- `aggregated_data.csv`: per-configuration statistics (mean, stddev, median, CV, 95% CI)
- `metadata.json`: machine specs, runtime config, timestamp, commit hash
- `graphs/`: publication-quality PNG figures
- `run_data/`: detailed per-run logs and raw benchmark JSON
- `anomaly_investigation/`: sharded-2key/c500 rerun with debug logging, CPU traces, and throughput traces

## Graphs

- `graphs/throughput_vs_concurrency.png`
- `graphs/latency_vs_concurrency.png`
- `graphs/cv_vs_concurrency.png`
- `graphs/throughput_distribution.png`

## Outlier/Anomaly Decision

- Decision: no_formal_outlier
- Details: Rerun anomaly did not repeat and original run does not violate |value| > 3x median rule.
- Excluded runs for aggregated analysis: []

## Raw Logs

Detailed run logs are available under:

- `/Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data/20260420_090739`
