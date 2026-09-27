# experiment_results_v12

This directory contains reproducible v12 experiment artifacts.

## Contents

- `raw_data.csv`: per-run data (`strategy, concurrency, run_id, throughput, p99`, plus additional diagnostics)
- `aggregated_data.csv`: per-configuration statistics (mean, stddev, median, CV, 95% CI)
- `metadata.json`: machine specs, runtime config, timestamp, commit hash
- `graphs/`: publication-quality PNG figures
- `anomaly_investigation/`: sharded-2key/c500 rerun with debug logging, CPU traces, and throughput traces

Note: the per-run raw logs and benchmark JSON previously kept under
`run_data/` (3059 files, ~83MB) have been removed from this working tree
as repository cleanup — they were already archived (compressed) in
`zenodo_package_upload/run_data.zip`, which is the canonical copy going
forward. `raw_data.csv` and `aggregated_data.csv` in this directory remain
the source of truth for analysis; nothing analytical was lost.

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

Detailed run logs for this data are archived in
`zenodo_package_upload/run_data.zip` (see the note above).
