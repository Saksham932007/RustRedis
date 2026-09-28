# Repository Structure

This file provides a compact, maintained view of the repository layout.

## Top Level

- Cargo.toml / Cargo.lock: Rust workspace (server + benchmarks)
- README.md: start here
- repo_structure.md: this file
- src/: server source
- benchmarks/: benchmark client + experiment orchestration/analysis scripts
- docs/: protocol docs, design notes, paper draft
- reports/: written-up experiment reports (one per project version)
- figures/: publication figure set
- results/: legacy (pre-v12) raw benchmark trees
- experiment_results_v12/, experiment_results_rmit/, experiment_results_rmit_azure/,
  experiment_results_rmit_advanced/: per-experiment-version datasets
  (aggregated CSV/JSON + analysis outputs)
- zenodo_package_upload/: packaged Zenodo deposition (zips) for the v12 dataset
- Provenance files kept at root (see "Historical/provenance files" below):
  final_experiment_v12.json, final_experiment_summary.md, v15_evidence.json,
  generate_publication_graphs.py, zenodo_deposition_package_plan_v16.md

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
- benchmarks/run_rmit_experiment.py: current randomized-interleaved-trials experiment runner (strategy x concurrency x workload, --workloads mixed/read-heavy/write-heavy)
- benchmarks/analyze_rmit_results.py: median+CI / two-state / paired-ratio analysis for RMIT data
- benchmarks/system_state.py: per-run machine-state snapshot (CPU freq, temp, memory, load, power)
- benchmarks/hardware_hypothesis_check.sh: thermal/scheduling/memory diagnostic script
- benchmarks/generate_publication_graphs.py: figure generation for the v12 dataset
- benchmarks/run_final_experiment_v12.py: legacy fixed-order runner, superseded by run_rmit_experiment.py
- benchmarks/run_final_matrix.sh, run_macos_m2_research.sh, run_paper_final_experiment.sh: legacy, superseded
- benchmarks/generate_final_experiment_report.py

## Documentation

- docs/rmit_experiment_protocol.md: current experiment protocol (laptop + cloud-VM paths, RMIT design)
- docs/paper_draft.md: paper draft, filled in with both RMIT datasets' results
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
- reports/v15_change_and_rationale.md

## Figures

- figures/canonical/: single canonical publication figure set
  - throughput_vs_concurrency.png
  - latency_vs_concurrency.png
  - cv_vs_concurrency.png
  - throughput_distribution.png

## Experiment datasets

- experiment_results_v12/: v12 fixed-order matrix (superseded design; kept for
  provenance and the before/after comparison in docs/paper_draft.md). Per-run
  raw logs were removed from the working tree (already archived compressed in
  zenodo_package_upload/run_data.zip) — only aggregated CSV/JSON remain.
- experiment_results_rmit/: current design, laptop hardware (i3-10110U)
- experiment_results_rmit_azure/: current design, cloud VM (Azure Standard_D4s_v6), mixed workload only — the first clean-room dataset
- experiment_results_rmit_advanced/: current design, cloud VM (Azure Standard_D8s_v6), 3 workload types x concurrency up to 3000 — the largest and most complete dataset
- results/: pre-v12 raw benchmark trees (final_experiment, final_matrix,
  macos_m2, metrics_strategy_mandatory, system_validation_v15) — legacy,
  kept as-is for provenance; not part of the current experiment pipeline.

## Historical/provenance files

A handful of files live at repo root rather than in a subdirectory because
they're cross-referenced by several other documents (`reports/v15_change_and_rationale.md`,
`zenodo_deposition_package_plan_v16.md`, `zenodo_package_upload/README.md`,
`benchmarks/generate_final_experiment_v12_dataset.py`'s default output path)
and moving them would require updating all of those in lockstep. They're
frozen provenance records for already-completed work, not part of the active
pipeline:

- `final_experiment_v12.json`: full structured v12 dataset dump
- `final_experiment_summary.md`: generated summary pointing at the above
- `v15_evidence.json`: causal-validation evidence backing `reports/v15_change_and_rationale.md`
- `zenodo_deposition_package_plan_v16.md`: the plan that produced `zenodo_package_upload/` (deposition already completed)
