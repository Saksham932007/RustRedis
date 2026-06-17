# RustRedis Zenodo Reproducibility Package

## 1. Overview

This package is a clean, minimal, publication-ready artifact set for reproducing the RustRedis systems paper experiments.

Included artifact groups:
- source/: Rust server and benchmark source code required to rebuild binaries.
- experiment_results_v12/: raw and aggregated benchmark outputs, run-level logs, and validation evidence.
- processed_dataset/: final processed dataset and summary markdown.
- scripts/: experiment orchestration and analysis scripts.
- figures/: publication figure files.
- docs/: protocol and design documentation.
- reports/: provenance and rationale records.

## 2. Quick Reproduction (single command)

Run from the package root:

    python3 scripts/run_final_experiment_v12.py \
      --output-dir experiment_results_v12 \
      --runs 30 \
      --requests-per-client 1000 \
      --key-space 10000 \
      --value-size 64

## 3. Full Repro Steps

1. Install Python dependencies.

    python3 -m pip install -r requirements-zenodo.txt

2. Build Rust binaries from the packaged source tree.

    cd source
    cargo build --release --bin server
    cargo build --release --manifest-path benchmarks/Cargo.toml
    cd ..

3. Run the full v12 benchmark matrix.

    python3 scripts/run_final_experiment_v12.py \
      --output-dir experiment_results_v12 \
      --runs 30 \
      --requests-per-client 1000 \
      --key-space 10000 \
      --value-size 64

4. Regenerate processed dataset artifacts.

    python3 scripts/generate_final_experiment_v12_dataset.py \
      --root . \
      --raw-data experiment_results_v12/raw_data.csv \
      --metadata experiment_results_v12/metadata.json \
      --validation-latest experiment_results_v12/system_validation/latest_run.txt \
      --output-json processed_dataset/final_experiment_v12.json \
      --output-md processed_dataset/final_experiment_summary.md \
      --config-output experiment_results_v12/final_experiment_config.json

5. Optionally regenerate a narrative report for one run_id.

    cd source
    python3 ../scripts/generate_final_experiment_report.py \
      --input ../experiment_results_v12/run_data/<run_id> \
      --output ../reports/final_experiment_report.md
    cd ..

## 4. Folder Structure

    zenodo_package/
    |- experiment_results_v12/
    |  |- raw_data.csv
    |  |- aggregated_data.csv
    |  |- metadata.json
    |  |- final_experiment_config.json
    |  |- anomaly_decision.json
    |  |- run_data/
    |  |- system_validation/
    |  |- anomaly_investigation/
    |- processed_dataset/
    |  |- final_experiment_v12.json
    |  |- final_experiment_summary.md
    |- scripts/
    |- source/
    |  |- Cargo.toml
    |  |- Cargo.lock
    |  |- src/
    |  |- benchmarks/
    |- docs/
    |- reports/
    |- figures/
    |  |- figure1_throughput.png
    |  |- figure2_latency.png
    |  |- figure3_distribution.png
    |  |- figure4_cv.png
    |- LICENSE
    |- requirements-zenodo.txt
    |- README.md

## 5. Provenance (commit window)

Commit window used in paper lineage:
- be472595525c361656ca88f3ed2d2861105083f0
- 624cb56a4e3e9dc0f7fde6cc047085a330a28d21
- 0c6384c1a22fec745d3e058bd2d37471a10c5b01
- 1a605ed40b6870e6a16be0ed990b071b268985a2

Provenance note:
- metadata.json contains an anchor commit hash.
- The final dataset lineage spans the commit window above.

## 6. Data description

- experiment_results_v12/raw_data.csv: per-run throughput and latency measurements per strategy/concurrency/repetition.
- experiment_results_v12/aggregated_data.csv: aggregate means, standard deviations, confidence intervals, and CV values.
- experiment_results_v12/run_data/: full per-configuration logs, JSON outputs, and benchmark traces.
- experiment_results_v12/system_validation/: mechanism-specific validation logs (sharding, flush counters, histogram behavior).
- experiment_results_v12/anomaly_investigation/: targeted reruns and evidence for anomaly decisions.
- processed_dataset/final_experiment_v12.json: publication-ready structured analysis dataset.
- processed_dataset/final_experiment_summary.md: compact textual summary of key outcomes.

## 7. Figure mapping

- Figure 1 (throughput vs concurrency): figures/figure1_throughput.png
- Figure 2 (latency vs concurrency): figures/figure2_latency.png
- Figure 3 (throughput distribution): figures/figure3_distribution.png
- Figure 4 (CV vs concurrency): figures/figure4_cv.png
