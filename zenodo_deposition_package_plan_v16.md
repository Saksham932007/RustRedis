# Zenodo Deposition Package Plan (v16)

Repository root: <repo-root>
Date: 2026-04-22

## 1) File List (complete)

### SOURCE CODE
1. <repo-root>/Cargo.toml
2. <repo-root>/Cargo.lock
3. <repo-root>/src/lib.rs
4. <repo-root>/src/main.rs
5. <repo-root>/src/bin/server.rs
6. <repo-root>/src/command_metrics.rs
7. <repo-root>/src/metrics.rs
8. <repo-root>/src/connection.rs
9. <repo-root>/src/frame.rs
10. <repo-root>/src/db.rs
11. <repo-root>/src/db_dashmap.rs
12. <repo-root>/src/persistence.rs
13. <repo-root>/src/pubsub.rs
14. <repo-root>/src/cmd
15. <repo-root>/src/db
16. <repo-root>/benchmarks/Cargo.toml
17. <repo-root>/benchmarks/src/main.rs

### ANALYSIS
1. <repo-root>/benchmarks/generate_final_experiment_v12_dataset.py
2. <repo-root>/benchmarks/generate_final_experiment_report.py
3. <repo-root>/benchmarks/analysis.py
4. <repo-root>/benchmarks/aggregate_final_matrix.py
5. <repo-root>/benchmarks/summarize_macos_m2_results.py
6. <repo-root>/generate_publication_graphs.py
7. <repo-root>/final_experiment_v12.json
8. <repo-root>/final_experiment_summary.md
9. <repo-root>/reports/final_experiment_report.md
10. <repo-root>/reports/final_experiment_report_enhanced.md
11. <repo-root>/reports/final_experiment_details.md

### BENCHMARK EXECUTION SCRIPTS
1. <repo-root>/benchmarks/run_final_experiment_v12.py
2. <repo-root>/benchmarks/run_paper_final_experiment.sh
3. <repo-root>/benchmarks/run_final_matrix.sh
4. <repo-root>/benchmarks/run_macos_m2_research.sh
5. <repo-root>/benchmarks/run_perf.sh

### DATA
1. <repo-root>/experiment_results_v12/raw_data.csv
2. <repo-root>/experiment_results_v12/aggregated_data.csv
3. <repo-root>/final_experiment_v12.json
4. <repo-root>/experiment_results_v12/run_data
5. <repo-root>/experiment_results_v12/run_data/20260420_090427
6. <repo-root>/experiment_results_v12/run_data/20260420_090739
7. <repo-root>/experiment_results_v12/system_validation
8. <repo-root>/experiment_results_v12/system_validation/latest_run.txt
9. <repo-root>/experiment_results_v12/system_validation/20260421_170502
10. <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt
11. <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt
12. <repo-root>/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt
13. <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c400/cmdstat.txt
14. <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt
15. <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/stats.txt
16. <repo-root>/experiment_results_v12/anomaly_investigation
17. <repo-root>/experiment_results_v12/anomaly_decision.json

### CONFIG
1. <repo-root>/experiment_results_v12/metadata.json
2. <repo-root>/experiment_results_v12/final_experiment_config.json
3. <repo-root>/Cargo.toml
4. <repo-root>/benchmarks/Cargo.toml

### FIGURES (FINAL VERSION, 300 DPI VERIFIED)
1. <repo-root>/experiment_results_v12/graphs/throughput_vs_concurrency.png
2. <repo-root>/experiment_results_v12/graphs/latency_vs_concurrency.png
3. <repo-root>/experiment_results_v12/graphs/throughput_distribution.png
4. <repo-root>/experiment_results_v12/graphs/cv_vs_concurrency.png

### SUPPORTING ARTIFACT
1. <repo-root>/experiment_results_v12/README.md
2. <repo-root>/v15_evidence.json
3. <repo-root>/reports/v15_change_and_rationale.md
4. <repo-root>/docs/system-design.md
5. <repo-root>/docs/failure-analysis.md
6. <repo-root>/docs/macos_m2_experiment_protocol.md
7. <repo-root>/README.md
8. <repo-root>/repo_structure.md

## 2) Missing Items (if any)

1. MISSING: <repo-root>/zenodo_package/LICENSE
- How to generate: add MIT license text as LICENSE.
- Produced by: manual creation.

2. MISSING: <repo-root>/zenodo_package/README.md
- How to generate: use Section 4 content from this plan.
- Produced by: manual creation.

3. MISSING: <repo-root>/zenodo_package/requirements-zenodo.txt
- How to generate: include Python dependencies used by scripts (matplotlib, scipy, numpy).
- Produced by: manual creation.

## 3) Folder Structure

All paths below are absolute target paths for the Zenodo package:

<repo-root>/zenodo_package/
<repo-root>/zenodo_package/experiment_results_v12/
<repo-root>/zenodo_package/experiment_results_v12/raw_data.csv
<repo-root>/zenodo_package/experiment_results_v12/aggregated_data.csv
<repo-root>/zenodo_package/experiment_results_v12/metadata.json
<repo-root>/zenodo_package/experiment_results_v12/final_experiment_config.json
<repo-root>/zenodo_package/experiment_results_v12/anomaly_decision.json
<repo-root>/zenodo_package/experiment_results_v12/README.md
<repo-root>/zenodo_package/experiment_results_v12/run_data/
<repo-root>/zenodo_package/experiment_results_v12/run_data/20260420_090427/
<repo-root>/zenodo_package/experiment_results_v12/run_data/20260420_090739/
<repo-root>/zenodo_package/experiment_results_v12/system_validation/
<repo-root>/zenodo_package/experiment_results_v12/system_validation/latest_run.txt
<repo-root>/zenodo_package/experiment_results_v12/system_validation/20260421_170502/
<repo-root>/zenodo_package/experiment_results_v12/anomaly_investigation/
<repo-root>/zenodo_package/experiment_results_v12/graphs/
<repo-root>/zenodo_package/experiment_results_v12/graphs/throughput_vs_concurrency.png
<repo-root>/zenodo_package/experiment_results_v12/graphs/latency_vs_concurrency.png
<repo-root>/zenodo_package/experiment_results_v12/graphs/throughput_distribution.png
<repo-root>/zenodo_package/experiment_results_v12/graphs/cv_vs_concurrency.png
<repo-root>/zenodo_package/processed_dataset/
<repo-root>/zenodo_package/processed_dataset/final_experiment_v12.json
<repo-root>/zenodo_package/processed_dataset/final_experiment_summary.md
<repo-root>/zenodo_package/scripts/
<repo-root>/zenodo_package/scripts/run_final_experiment_v12.py
<repo-root>/zenodo_package/scripts/generate_final_experiment_v12_dataset.py
<repo-root>/zenodo_package/scripts/generate_final_experiment_report.py
<repo-root>/zenodo_package/scripts/generate_publication_graphs.py
<repo-root>/zenodo_package/scripts/run_paper_final_experiment.sh
<repo-root>/zenodo_package/scripts/run_final_matrix.sh
<repo-root>/zenodo_package/scripts/run_macos_m2_research.sh
<repo-root>/zenodo_package/scripts/run_perf.sh
<repo-root>/zenodo_package/scripts/analysis.py
<repo-root>/zenodo_package/scripts/aggregate_final_matrix.py
<repo-root>/zenodo_package/scripts/summarize_macos_m2_results.py
<repo-root>/zenodo_package/source/
<repo-root>/zenodo_package/source/Cargo.toml
<repo-root>/zenodo_package/source/Cargo.lock
<repo-root>/zenodo_package/source/benchmarks/Cargo.toml
<repo-root>/zenodo_package/source/src/
<repo-root>/zenodo_package/source/benchmarks/src/main.rs
<repo-root>/zenodo_package/docs/
<repo-root>/zenodo_package/docs/system-design.md
<repo-root>/zenodo_package/docs/failure-analysis.md
<repo-root>/zenodo_package/docs/macos_m2_experiment_protocol.md
<repo-root>/zenodo_package/reports/
<repo-root>/zenodo_package/reports/v15_change_and_rationale.md
<repo-root>/zenodo_package/reports/v15_evidence.json
<repo-root>/zenodo_package/README.md
<repo-root>/zenodo_package/LICENSE
<repo-root>/zenodo_package/requirements-zenodo.txt

## 4) README.md (full content)

# RustRedis Systems Paper v16 Reproducibility Package

## Overview

This Zenodo deposition contains the complete reproducibility package for the RustRedis systems research paper (v16), including:

- Raw per-run benchmark data
- Aggregated statistics used in analysis
- Processed final dataset JSON
- Run-level logs and system-validation evidence
- Figure artifacts for Figure 1 through Figure 4
- Experiment orchestration, dataset-generation, and report-generation scripts
- Source code for the metrics strategy implementations

Primary data artifacts:
- <repo-root>/experiment_results_v12/raw_data.csv
- <repo-root>/experiment_results_v12/aggregated_data.csv
- <repo-root>/final_experiment_v12.json
- <repo-root>/experiment_results_v12/run_data
- <repo-root>/experiment_results_v12/system_validation

## Reproducibility Instructions

### Prerequisites

- Rust and cargo toolchain
- redis-cli
- Python 3
- Python packages: matplotlib, scipy, numpy

### Build binaries

Run from repository root:

cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml

### Run full v12 experiment pipeline

python3 <repo-root>/benchmarks/run_final_experiment_v12.py --output-dir experiment_results_v12 --runs 30 --requests-per-client 1000 --key-space 10000 --value-size 64

This command regenerates:
- <repo-root>/experiment_results_v12/raw_data.csv
- <repo-root>/experiment_results_v12/aggregated_data.csv
- <repo-root>/experiment_results_v12/graphs
- <repo-root>/experiment_results_v12/metadata.json
- <repo-root>/experiment_results_v12/anomaly_investigation

### Regenerate processed dataset and config

python3 <repo-root>/benchmarks/generate_final_experiment_v12_dataset.py --root <repo-root> --raw-data experiment_results_v12/raw_data.csv --metadata experiment_results_v12/metadata.json --validation-latest experiment_results_v12/system_validation/latest_run.txt --output-json final_experiment_v12.json --output-md final_experiment_summary.md --config-output experiment_results_v12/final_experiment_config.json

This command regenerates:
- <repo-root>/final_experiment_v12.json
- <repo-root>/final_experiment_summary.md
- <repo-root>/experiment_results_v12/final_experiment_config.json

### Regenerate report artifact

python3 <repo-root>/benchmarks/generate_final_experiment_report.py --input <repo-root>/results/final_experiment/<timestamp> --output <repo-root>/reports/final_experiment_report.md

### Reproduce figures

Figure files are generated as part of the v12 pipeline and stored in:
- <repo-root>/experiment_results_v12/graphs/throughput_vs_concurrency.png
- <repo-root>/experiment_results_v12/graphs/latency_vs_concurrency.png
- <repo-root>/experiment_results_v12/graphs/throughput_distribution.png
- <repo-root>/experiment_results_v12/graphs/cv_vs_concurrency.png

## Provenance

Commit window used for this archived dataset lineage:
- be472595525c361656ca88f3ed2d2861105083f0
- 624cb56a4e3e9dc0f7fde6cc047085a330a28d21
- 0c6384c1a22fec745d3e058bd2d37471a10c5b01
- 1a605ed40b6870e6a16be0ed990b071b268985a2

Provenance note:
metadata.json shows anchor commit, but full dataset was produced across multiple commits.

Reproducibility statement:
archived scripts reconstruct full dataset deterministically.

Supporting provenance evidence:
- <repo-root>/reports/v15_change_and_rationale.md
- <repo-root>/v15_evidence.json
- <repo-root>/experiment_results_v12/metadata.json

## Figure Mapping

- Figure 1: <repo-root>/experiment_results_v12/graphs/throughput_vs_concurrency.png
- Figure 2: <repo-root>/experiment_results_v12/graphs/latency_vs_concurrency.png
- Figure 3: <repo-root>/experiment_results_v12/graphs/throughput_distribution.png
- Figure 4: <repo-root>/experiment_results_v12/graphs/cv_vs_concurrency.png

## System Validation Evidence Included

- Shard distribution logs:
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt
- ThreadLocal flush counters:
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt
- HdrHistogram profiling logs:
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c400/cmdstat.txt
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt
  - <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/stats.txt

## 5) Traceability Table

| Paper Section | Required Artifact | File |
|---|---|---|
| Section 3 (Implementation) | Metrics strategy implementation | <repo-root>/src/command_metrics.rs |
| Section 3 (Server integration) | Runtime wiring of metrics strategies and flush tasks | <repo-root>/src/bin/server.rs |
| Section 3.3.3 (Mechanism validation) | Sharded-N shard distribution evidence | <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | Sharded-2key shard contention evidence | <repo-root>/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | ThreadLocal flush counters | <repo-root>/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | HdrHistogram phase-swap and trigger counters | <repo-root>/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt |
| Section 4 (Experimental setup) | Orchestration script | <repo-root>/benchmarks/run_final_experiment_v12.py |
| Section 4 (Experimental setup) | Runtime metadata and machine spec | <repo-root>/experiment_results_v12/metadata.json |
| Section 4 (Experimental setup) | Final experiment configuration | <repo-root>/experiment_results_v12/final_experiment_config.json |
| Section 5 (Results) | Per-run raw matrix data | <repo-root>/experiment_results_v12/raw_data.csv |
| Section 5 (Results) | Aggregated means, SD, CI, CV | <repo-root>/experiment_results_v12/aggregated_data.csv |
| Section 5 (Results + statistics) | Processed final dataset with sectioned analysis blocks | <repo-root>/final_experiment_v12.json |
| Figures 1-4 | Final figure artifacts | <repo-root>/experiment_results_v12/graphs/throughput_vs_concurrency.png; <repo-root>/experiment_results_v12/graphs/latency_vs_concurrency.png; <repo-root>/experiment_results_v12/graphs/throughput_distribution.png; <repo-root>/experiment_results_v12/graphs/cv_vs_concurrency.png |
| Reproducibility appendix | Full run-level evidence | <repo-root>/experiment_results_v12/run_data |
| Provenance statement | Commit-window rationale and provenance integrity | <repo-root>/reports/v15_change_and_rationale.md; <repo-root>/v15_evidence.json |

## 6) Final Checklist

- [x] Raw data present
- [x] Aggregated stats reproducible
- [x] Scripts executable
- [x] Figures match paper
- [x] Provenance documented
- [ ] No missing files
