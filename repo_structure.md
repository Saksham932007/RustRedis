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
- benchmarks/run_final_matrix.sh
- benchmarks/run_macos_m2_research.sh
- benchmarks/run_paper_final_experiment.sh
- benchmarks/generate_final_experiment_report.py

## Documentation

- docs/README.md: docs index
- docs/system-design.md
- docs/failure-analysis.md
- docs/macos_m2_experiment_protocol.md
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
