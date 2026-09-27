# Repository Structure

This file provides a compact, maintained view of the repository layout.

## Top Level

- Cargo.toml
- Cargo.lock
- README.md
- repo_structure.md
- appendonly.aof
- src/
- benchmarks/
- docs/
- reports/
- figures/
- results/

## Source Code

- src/bin/server.rs: main server binary entrypoint
- src/cmd/: command parsing and execution
- src/db.rs: mutex-backed database backend
- src/db_dashmap.rs: sharded database backend
- src/command_metrics.rs: observability strategy implementations
- src/persistence.rs: AOF persistence/replay
- src/connection.rs, src/frame.rs, src/pubsub.rs, src/metrics.rs

## Benchmarking

- benchmarks/src/main.rs: benchmark client
- benchmarks/run_rmit_experiment.py: current randomized-interleaved-trials experiment runner
- benchmarks/analyze_rmit_results.py: median+CI / two-state / paired-ratio analysis for RMIT data
- benchmarks/system_state.py: per-run machine-state snapshot (CPU freq, temp, memory, load, power)
- benchmarks/hardware_hypothesis_check.sh: thermal/scheduling/memory diagnostic script
- benchmarks/run_final_matrix.sh: legacy, superseded
- benchmarks/run_macos_m2_research.sh: legacy, superseded (M2 hardware)
- benchmarks/run_paper_final_experiment.sh: legacy, superseded
- benchmarks/run_final_experiment_v12.py: legacy fixed-order runner, superseded by run_rmit_experiment.py
- benchmarks/generate_final_experiment_report.py

## Documentation

- docs/rmit_experiment_protocol.md: current experiment protocol (i3-10110U/8GB laptop, RMIT design)
- docs/README.md: docs index
- docs/system-design.md
- docs/failure-analysis.md
- docs/macos_m2_experiment_protocol.md: legacy, superseded (M2 hardware)
- docs/legacy_docs_archive.md

## Reports

- reports/final_experiment_v5.md
- reports/final_experiment_report.md
- reports/final_experiment_report_enhanced.md
- reports/final_experiment_details.md

## Figures

- figures/canonical/: single canonical publication figure set
  - throughput_vs_concurrency.png
  - latency_vs_concurrency.png
  - cv_vs_concurrency.png
  - throughput_distribution.png

## Results

- results/final_experiment_v5/
- results/final_experiment/
- results/final_matrix/
- results/macos_m2/
- results/metrics_strategy_mandatory/
