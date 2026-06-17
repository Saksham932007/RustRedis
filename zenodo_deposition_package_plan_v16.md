# Zenodo Deposition Package Plan (v16)

Repository root: /Users/sakshamkapoor/Projects/RustRedis
Date: 2026-04-22

## 1) File List (complete)

### SOURCE CODE
1. /Users/sakshamkapoor/Projects/RustRedis/Cargo.toml
2. /Users/sakshamkapoor/Projects/RustRedis/Cargo.lock
3. /Users/sakshamkapoor/Projects/RustRedis/src/lib.rs
4. /Users/sakshamkapoor/Projects/RustRedis/src/main.rs
5. /Users/sakshamkapoor/Projects/RustRedis/src/bin/server.rs
6. /Users/sakshamkapoor/Projects/RustRedis/src/command_metrics.rs
7. /Users/sakshamkapoor/Projects/RustRedis/src/metrics.rs
8. /Users/sakshamkapoor/Projects/RustRedis/src/connection.rs
9. /Users/sakshamkapoor/Projects/RustRedis/src/frame.rs
10. /Users/sakshamkapoor/Projects/RustRedis/src/db.rs
11. /Users/sakshamkapoor/Projects/RustRedis/src/db_dashmap.rs
12. /Users/sakshamkapoor/Projects/RustRedis/src/persistence.rs
13. /Users/sakshamkapoor/Projects/RustRedis/src/pubsub.rs
14. /Users/sakshamkapoor/Projects/RustRedis/src/cmd
15. /Users/sakshamkapoor/Projects/RustRedis/src/db
16. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/Cargo.toml
17. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/src/main.rs

### ANALYSIS
1. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/generate_final_experiment_v12_dataset.py
2. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/generate_final_experiment_report.py
3. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/analysis.py
4. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/aggregate_final_matrix.py
5. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/summarize_macos_m2_results.py
6. /Users/sakshamkapoor/Projects/RustRedis/generate_publication_graphs.py
7. /Users/sakshamkapoor/Projects/RustRedis/final_experiment_v12.json
8. /Users/sakshamkapoor/Projects/RustRedis/final_experiment_summary.md
9. /Users/sakshamkapoor/Projects/RustRedis/reports/final_experiment_report.md
10. /Users/sakshamkapoor/Projects/RustRedis/reports/final_experiment_report_enhanced.md
11. /Users/sakshamkapoor/Projects/RustRedis/reports/final_experiment_details.md

### BENCHMARK EXECUTION SCRIPTS
1. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_final_experiment_v12.py
2. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_paper_final_experiment.sh
3. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_final_matrix.sh
4. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_macos_m2_research.sh
5. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_perf.sh

### DATA
1. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/raw_data.csv
2. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/aggregated_data.csv
3. /Users/sakshamkapoor/Projects/RustRedis/final_experiment_v12.json
4. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data
5. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data/20260420_090427
6. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data/20260420_090739
7. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation
8. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/latest_run.txt
9. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502
10. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt
11. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt
12. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt
13. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c400/cmdstat.txt
14. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt
15. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/stats.txt
16. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/anomaly_investigation
17. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/anomaly_decision.json

### CONFIG
1. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/metadata.json
2. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/final_experiment_config.json
3. /Users/sakshamkapoor/Projects/RustRedis/Cargo.toml
4. /Users/sakshamkapoor/Projects/RustRedis/benchmarks/Cargo.toml

### FIGURES (FINAL VERSION, 300 DPI VERIFIED)
1. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_vs_concurrency.png
2. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/latency_vs_concurrency.png
3. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_distribution.png
4. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/cv_vs_concurrency.png

### SUPPORTING ARTIFACT
1. /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/README.md
2. /Users/sakshamkapoor/Projects/RustRedis/v15_evidence.json
3. /Users/sakshamkapoor/Projects/RustRedis/reports/v15_change_and_rationale.md
4. /Users/sakshamkapoor/Projects/RustRedis/docs/system-design.md
5. /Users/sakshamkapoor/Projects/RustRedis/docs/failure-analysis.md
6. /Users/sakshamkapoor/Projects/RustRedis/docs/macos_m2_experiment_protocol.md
7. /Users/sakshamkapoor/Projects/RustRedis/README.md
8. /Users/sakshamkapoor/Projects/RustRedis/repo_structure.md

## 2) Missing Items (if any)

1. MISSING: /Users/sakshamkapoor/Projects/RustRedis/zenodo_package/LICENSE
- How to generate: add MIT license text as LICENSE.
- Produced by: manual creation.

2. MISSING: /Users/sakshamkapoor/Projects/RustRedis/zenodo_package/README.md
- How to generate: use Section 4 content from this plan.
- Produced by: manual creation.

3. MISSING: /Users/sakshamkapoor/Projects/RustRedis/zenodo_package/requirements-zenodo.txt
- How to generate: include Python dependencies used by scripts (matplotlib, scipy, numpy).
- Produced by: manual creation.

## 3) Folder Structure

All paths below are absolute target paths for the Zenodo package:

/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/raw_data.csv
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/aggregated_data.csv
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/metadata.json
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/final_experiment_config.json
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/anomaly_decision.json
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/README.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/run_data/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/run_data/20260420_090427/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/run_data/20260420_090739/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/system_validation/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/system_validation/latest_run.txt
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/system_validation/20260421_170502/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/anomaly_investigation/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/graphs/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/graphs/throughput_vs_concurrency.png
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/graphs/latency_vs_concurrency.png
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/graphs/throughput_distribution.png
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/experiment_results_v12/graphs/cv_vs_concurrency.png
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/processed_dataset/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/processed_dataset/final_experiment_v12.json
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/processed_dataset/final_experiment_summary.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/run_final_experiment_v12.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/generate_final_experiment_v12_dataset.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/generate_final_experiment_report.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/generate_publication_graphs.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/run_paper_final_experiment.sh
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/run_final_matrix.sh
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/run_macos_m2_research.sh
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/run_perf.sh
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/analysis.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/aggregate_final_matrix.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/scripts/summarize_macos_m2_results.py
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/Cargo.toml
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/Cargo.lock
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/benchmarks/Cargo.toml
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/src/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/source/benchmarks/src/main.rs
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/docs/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/docs/system-design.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/docs/failure-analysis.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/docs/macos_m2_experiment_protocol.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/reports/
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/reports/v15_change_and_rationale.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/reports/v15_evidence.json
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/README.md
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/LICENSE
/Users/sakshamkapoor/Projects/RustRedis/zenodo_package/requirements-zenodo.txt

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
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/raw_data.csv
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/aggregated_data.csv
- /Users/sakshamkapoor/Projects/RustRedis/final_experiment_v12.json
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation

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

python3 /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_final_experiment_v12.py --output-dir experiment_results_v12 --runs 30 --requests-per-client 1000 --key-space 10000 --value-size 64

This command regenerates:
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/raw_data.csv
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/aggregated_data.csv
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/metadata.json
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/anomaly_investigation

### Regenerate processed dataset and config

python3 /Users/sakshamkapoor/Projects/RustRedis/benchmarks/generate_final_experiment_v12_dataset.py --root /Users/sakshamkapoor/Projects/RustRedis --raw-data experiment_results_v12/raw_data.csv --metadata experiment_results_v12/metadata.json --validation-latest experiment_results_v12/system_validation/latest_run.txt --output-json final_experiment_v12.json --output-md final_experiment_summary.md --config-output experiment_results_v12/final_experiment_config.json

This command regenerates:
- /Users/sakshamkapoor/Projects/RustRedis/final_experiment_v12.json
- /Users/sakshamkapoor/Projects/RustRedis/final_experiment_summary.md
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/final_experiment_config.json

### Regenerate report artifact

python3 /Users/sakshamkapoor/Projects/RustRedis/benchmarks/generate_final_experiment_report.py --input /Users/sakshamkapoor/Projects/RustRedis/results/final_experiment/<timestamp> --output /Users/sakshamkapoor/Projects/RustRedis/reports/final_experiment_report.md

### Reproduce figures

Figure files are generated as part of the v12 pipeline and stored in:
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_vs_concurrency.png
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/latency_vs_concurrency.png
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_distribution.png
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/cv_vs_concurrency.png

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
- /Users/sakshamkapoor/Projects/RustRedis/reports/v15_change_and_rationale.md
- /Users/sakshamkapoor/Projects/RustRedis/v15_evidence.json
- /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/metadata.json

## Figure Mapping

- Figure 1: /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_vs_concurrency.png
- Figure 2: /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/latency_vs_concurrency.png
- Figure 3: /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_distribution.png
- Figure 4: /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/cv_vs_concurrency.png

## System Validation Evidence Included

- Shard distribution logs:
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt
- ThreadLocal flush counters:
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt
- HdrHistogram profiling logs:
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c400/cmdstat.txt
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt
  - /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/stats.txt

## 5) Traceability Table

| Paper Section | Required Artifact | File |
|---|---|---|
| Section 3 (Implementation) | Metrics strategy implementation | /Users/sakshamkapoor/Projects/RustRedis/src/command_metrics.rs |
| Section 3 (Server integration) | Runtime wiring of metrics strategies and flush tasks | /Users/sakshamkapoor/Projects/RustRedis/src/bin/server.rs |
| Section 3.3.3 (Mechanism validation) | Sharded-N shard distribution evidence | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_n_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | Sharded-2key shard contention evidence | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/sharded_2key_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | ThreadLocal flush counters | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/thread_local_c500/cmdstat.txt |
| Section 3.3.3 (Mechanism validation) | HdrHistogram phase-swap and trigger counters | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/system_validation/20260421_170502/hdr_histogram_c500/cmdstat.txt |
| Section 4 (Experimental setup) | Orchestration script | /Users/sakshamkapoor/Projects/RustRedis/benchmarks/run_final_experiment_v12.py |
| Section 4 (Experimental setup) | Runtime metadata and machine spec | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/metadata.json |
| Section 4 (Experimental setup) | Final experiment configuration | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/final_experiment_config.json |
| Section 5 (Results) | Per-run raw matrix data | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/raw_data.csv |
| Section 5 (Results) | Aggregated means, SD, CI, CV | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/aggregated_data.csv |
| Section 5 (Results + statistics) | Processed final dataset with sectioned analysis blocks | /Users/sakshamkapoor/Projects/RustRedis/final_experiment_v12.json |
| Figures 1-4 | Final figure artifacts | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_vs_concurrency.png; /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/latency_vs_concurrency.png; /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/throughput_distribution.png; /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/graphs/cv_vs_concurrency.png |
| Reproducibility appendix | Full run-level evidence | /Users/sakshamkapoor/Projects/RustRedis/experiment_results_v12/run_data |
| Provenance statement | Commit-window rationale and provenance integrity | /Users/sakshamkapoor/Projects/RustRedis/reports/v15_change_and_rationale.md; /Users/sakshamkapoor/Projects/RustRedis/v15_evidence.json |

## 6) Final Checklist

- [x] Raw data present
- [x] Aggregated stats reproducible
- [x] Scripts executable
- [x] Figures match paper
- [x] Provenance documented
- [ ] No missing files
