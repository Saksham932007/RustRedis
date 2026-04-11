# research_data.md

## 1. Overview

This document consolidates experimental artifacts, benchmark outputs, and structured measurements for the observability strategy study in RustRedis.

Research goal: **observability overhead under high concurrency**.

Primary dataset used for the full matrix:

- `results/final_matrix/20260408_154844/`

Additional datasets retained as supplementary evidence:

- `results/metrics_strategy_mandatory/20260408_010639/`
- `results/macos_m2/20260408_005543/`
- `results/final_matrix/20260408_154750/` (incomplete run)

## 2. Experimental Setup

### 2.1 Hardware

Primary matrix hardware metadata (`results/final_matrix/20260408_154844/machine_details.txt`):

- Host: `Sakshams-MacBook-Pro.local`
- CPU architecture: `arm64` (Darwin kernel `RELEASE_ARM64_T8112`)
- Logical CPUs: `8`
- Physical CPUs: `8`
- RAM: `8589934592` bytes (~8 GiB)
- OS: `macOS 26.4 (Build 25E246)`
- Kernel: `Darwin 25.4.0`
- Rust compiler: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Cargo: `cargo 1.94.1 (29ea6fb6a 2026-03-24)`

Runtime metadata from source:

- Tokio runtime flavor: `multi_thread` (`src/bin/server.rs`)
- Tokio crate version: `1.40` (`Cargo.toml`)

### 2.2 Workload

Primary matrix workload (`benchmarks/run_final_matrix.sh`, benchmark args, and matrix metadata):

- Workload type: `mixed`
- GET/SET ratio: `50% GET / 50% SET` (`benchmarks/src/main.rs`)
- Keyspace size: `10000`
- Value size: `64` bytes
- Concurrency levels: `100, 500, 1000`
- Core configurations: `4`, `8` (via `TOKIO_WORKER_THREADS`)
- Strategies: `disabled`, `global_mutex`, `sharded`, `thread_local`
- Runs per configuration: `3`

Request accounting details from benchmark implementation:

- CLI argument `--requests=10000` is divided by concurrency (`requests / conc`) per client thread.
- Effective total attempted operations per configuration are approximately `10000` (before errors).

### 2.3 Metrics Collected

From aggregated JSON and matrix summary files:

- Throughput: `ops_per_sec_mean`, `ops_per_sec_stddev`
- Latency: `p50_us_mean/stddev`, `p99_us_mean/stddev`
- Variance/stability: standard deviation and coefficient of variation (CV)
- Error counts: `total_errors` and per-run `errors`
- Overhead metrics (vs disabled baseline):
  - Throughput overhead percentage
  - p99 delta percentage
- Lock contention metrics (if available):
  - `cmdstat_lock_wait_us` (global mutex strategy only)
  - Derived contention estimate: `lock_wait_us / sum(total_time_us)`

## 3. Experiment Matrix

Primary matrix definition (`results/final_matrix/20260408_154844`):

Strategies:

- `disabled`
- `global_mutex`
- `sharded`
- `thread_local`

Core configurations:

- `4-core`
- `8-core`

Concurrency levels:

- `100`
- `500`
- `1000`

Total primary matrix cells:

- `2 cores x 4 strategies x 3 concurrencies = 24` configurations

## 4. Results (RAW + STRUCTURED)

### 4.1 Final Matrix Table

Source: `results/final_matrix/20260408_154844/observability_matrix_summary.csv` and `.md`

| Core   | Strategy     | Clients | Throughput mean +- stddev | p99 mean +- stddev (us) | Throughput CV | p99 CV | Errors | Throughput overhead vs disabled | p99 delta vs disabled |
| ------ | ------------ | ------: | ------------------------: | ----------------------: | ------------: | -----: | -----: | ------------------------------: | --------------------: |
| 4-core | disabled     |     100 |     143632.37 +- 12448.67 |       1741.67 +- 626.30 |         0.087 |  0.360 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |     100 |     138056.73 +- 25613.60 |       1727.33 +- 772.59 |         0.186 |  0.447 |      0 |                          +3.88% |                -0.82% |
| 4-core | sharded      |     100 |      90411.06 +- 39952.25 |      4952.67 +- 3063.43 |         0.442 |  0.619 |      0 |                         +37.05% |              +184.36% |
| 4-core | thread_local |     100 |      82123.91 +- 30970.99 |      9815.00 +- 9161.58 |         0.377 |  0.933 |      0 |                         +42.82% |              +463.54% |
| 4-core | disabled     |     500 |     125881.07 +- 45951.43 |       4109.00 +- 871.34 |         0.365 |  0.212 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |     500 |     114389.20 +- 34969.32 |       3441.00 +- 158.19 |         0.306 |  0.046 |      0 |                          +9.13% |               -16.26% |
| 4-core | sharded      |     500 |     106670.78 +- 57129.70 |      9307.67 +- 8193.05 |         0.536 |  0.880 |      0 |                         +15.26% |              +126.52% |
| 4-core | thread_local |     500 |      38272.14 +- 43719.03 |    38717.00 +- 63943.61 |         1.142 |  1.652 |  15960 |                         +69.60% |              +842.25% |
| 4-core | disabled     |    1000 |      128389.02 +- 1508.38 |       5285.00 +- 398.98 |         0.012 |  0.075 |      0 |                          +0.00% |                +0.00% |
| 4-core | global_mutex |    1000 |     105088.07 +- 18144.92 |       5521.67 +- 820.80 |         0.173 |  0.149 |      0 |                         +18.15% |                +4.48% |
| 4-core | sharded      |    1000 |      91352.13 +- 35453.37 |    20179.67 +- 26596.99 |         0.388 |  1.318 |      0 |                         +28.85% |              +281.83% |
| 4-core | thread_local |    1000 |              0.00 +- 0.00 |            0.00 +- 0.00 |         0.000 |  0.000 |  30000 |                        +100.00% |              -100.00% |
| 8-core | disabled     |     100 |     136655.09 +- 14670.27 |       1920.00 +- 831.38 |         0.107 |  0.433 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |     100 |     135929.94 +- 21008.12 |       1676.67 +- 480.22 |         0.155 |  0.286 |      0 |                          +0.53% |               -12.67% |
| 8-core | sharded      |     100 |     132813.11 +- 24009.25 |       2216.67 +- 973.90 |         0.181 |  0.439 |      0 |                          +2.81% |               +15.45% |
| 8-core | thread_local |     100 |     133243.34 +- 19785.69 |       1920.67 +- 852.34 |         0.148 |  0.444 |      0 |                          +2.50% |                +0.03% |
| 8-core | disabled     |     500 |     136904.73 +- 15681.26 |      5399.00 +- 1054.67 |         0.115 |  0.195 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |     500 |     116518.53 +- 23648.43 |      6698.33 +- 4937.92 |         0.203 |  0.737 |      0 |                         +14.89% |               +24.07% |
| 8-core | sharded      |     500 |     126927.29 +- 32057.34 |       4306.67 +- 795.95 |         0.253 |  0.185 |      0 |                          +7.29% |               -20.23% |
| 8-core | thread_local |     500 |      17860.39 +- 17564.46 |    71060.00 +- 87990.31 |         0.983 |  1.238 |  15980 |                         +86.95% |             +1216.17% |
| 8-core | disabled     |    1000 |      97064.92 +- 39319.73 |       6293.67 +- 509.51 |         0.405 |  0.081 |      0 |                          +0.00% |                +0.00% |
| 8-core | global_mutex |    1000 |     117447.35 +- 14885.43 |       5506.67 +- 503.51 |         0.127 |  0.091 |      0 |                         -21.00% |               -12.50% |
| 8-core | sharded      |    1000 |      103494.09 +- 9051.53 |      7680.67 +- 2505.74 |         0.087 |  0.326 |      0 |                          -6.62% |               +22.04% |
| 8-core | thread_local |    1000 |            11.26 +- 19.50 |   72800.00 +- 126093.30 |         1.732 |  1.732 |  29980 |                         +99.99% |             +1056.72% |

### 4.2 Key Graphs (described)

Observed/available graph artifact:

- `results/final_matrix/20260408_154844/observability_cost_vs_concurrency.png`
  - Contains 2x2 panels: throughput and p99 for 4-core and 8-core, each plotted across 100/500/1000 clients and four strategies.

Graph descriptions from structured data (`observability_matrix_summary.csv`):

- **Observability Cost vs Concurrency**
  - Throughput overhead relative to disabled is near zero to moderate for `global_mutex` and `sharded` in multiple cells.
  - `thread_local` shows high overhead and large error counts at `500` and `1000` clients.
- **Throughput Scaling**
  - Per-strategy throughput trajectories are provided by the table in 4.1 and can be plotted directly against client count.
- **Latency Scaling**
  - p99 trajectories are provided by the table in 4.1 and can be plotted directly against client count.

### 4.3 Per-Strategy Analysis (NO interpretation)

#### disabled

- 4-core:
  - c=100: throughput `143632.37 +- 12448.67`, p99 `1741.67 +- 626.30`, throughput CV `0.087`, p99 CV `0.360`, errors `0`
  - c=500: throughput `125881.07 +- 45951.43`, p99 `4109.00 +- 871.34`, throughput CV `0.365`, p99 CV `0.212`, errors `0`
  - c=1000: throughput `128389.02 +- 1508.38`, p99 `5285.00 +- 398.98`, throughput CV `0.012`, p99 CV `0.075`, errors `0`
- 8-core:
  - c=100: throughput `136655.09 +- 14670.27`, p99 `1920.00 +- 831.38`, throughput CV `0.107`, p99 CV `0.433`, errors `0`
  - c=500: throughput `136904.73 +- 15681.26`, p99 `5399.00 +- 1054.67`, throughput CV `0.115`, p99 CV `0.195`, errors `0`
  - c=1000: throughput `97064.92 +- 39319.73`, p99 `6293.67 +- 509.51`, throughput CV `0.405`, p99 CV `0.081`, errors `0`

#### global_mutex

- 4-core:
  - c=100: throughput `138056.73 +- 25613.60`, p99 `1727.33 +- 772.59`, throughput CV `0.186`, p99 CV `0.447`, errors `0`
  - c=500: throughput `114389.20 +- 34969.32`, p99 `3441.00 +- 158.19`, throughput CV `0.306`, p99 CV `0.046`, errors `0`
  - c=1000: throughput `105088.07 +- 18144.92`, p99 `5521.67 +- 820.80`, throughput CV `0.173`, p99 CV `0.149`, errors `0`
- 8-core:
  - c=100: throughput `135929.94 +- 21008.12`, p99 `1676.67 +- 480.22`, throughput CV `0.155`, p99 CV `0.286`, errors `0`
  - c=500: throughput `116518.53 +- 23648.43`, p99 `6698.33 +- 4937.92`, throughput CV `0.203`, p99 CV `0.737`, errors `0`
  - c=1000: throughput `117447.35 +- 14885.43`, p99 `5506.67 +- 503.51`, throughput CV `0.127`, p99 CV `0.091`, errors `0`

#### sharded

- 4-core:
  - c=100: throughput `90411.06 +- 39952.25`, p99 `4952.67 +- 3063.43`, throughput CV `0.442`, p99 CV `0.619`, errors `0`
  - c=500: throughput `106670.78 +- 57129.70`, p99 `9307.67 +- 8193.05`, throughput CV `0.536`, p99 CV `0.880`, errors `0`
  - c=1000: throughput `91352.13 +- 35453.37`, p99 `20179.67 +- 26596.99`, throughput CV `0.388`, p99 CV `1.318`, errors `0`
- 8-core:
  - c=100: throughput `132813.11 +- 24009.25`, p99 `2216.67 +- 973.90`, throughput CV `0.181`, p99 CV `0.439`, errors `0`
  - c=500: throughput `126927.29 +- 32057.34`, p99 `4306.67 +- 795.95`, throughput CV `0.253`, p99 CV `0.185`, errors `0`
  - c=1000: throughput `103494.09 +- 9051.53`, p99 `7680.67 +- 2505.74`, throughput CV `0.087`, p99 CV `0.326`, errors `0`

#### thread_local

- 4-core:
  - c=100: throughput `82123.91 +- 30970.99`, p99 `9815.00 +- 9161.58`, throughput CV `0.377`, p99 CV `0.933`, errors `0`
  - c=500: throughput `38272.14 +- 43719.03`, p99 `38717.00 +- 63943.61`, throughput CV `1.142`, p99 CV `1.652`, errors `15960`
  - c=1000: throughput `0.00 +- 0.00`, p99 `0.00 +- 0.00`, throughput CV `0.000`, p99 CV `0.000`, errors `30000`
- 8-core:
  - c=100: throughput `133243.34 +- 19785.69`, p99 `1920.67 +- 852.34`, throughput CV `0.148`, p99 CV `0.444`, errors `0`
  - c=500: throughput `17860.39 +- 17564.46`, p99 `71060.00 +- 87990.31`, throughput CV `0.983`, p99 CV `1.238`, errors `15980`
  - c=1000: throughput `11.26 +- 19.50`, p99 `72800.00 +- 126093.30`, throughput CV `1.732`, p99 CV `1.732`, errors `29980`

Raw per-run error evidence (thread_local):

- 4-core, c=500: per-run errors `[0, 5960, 10000]`, per-run total_ops `[10000, 4040, 0]`
- 4-core, c=1000: per-run errors `[10000, 10000, 10000]`, per-run total_ops `[0, 0, 0]`
- 8-core, c=500: per-run errors `[0, 5980, 10000]`, per-run total_ops `[10000, 4020, 0]`
- 8-core, c=1000: per-run errors `[10000, 9980, 10000]`, per-run total_ops `[0, 20, 0]`

### 4.4 Core Scaling Results

Computed from `observability_matrix_summary.csv` as `(8-core / 4-core - 1)`.

| Strategy     | Clients | Throughput delta (8c vs 4c) | p99 delta (8c vs 4c) | Throughput CV (4c -> 8c) | Errors (4c -> 8c) |
| ------------ | ------: | --------------------------: | -------------------: | -----------------------: | ----------------: |
| disabled     |     100 |                      -4.86% |              +10.24% |           0.087 -> 0.107 |            0 -> 0 |
| disabled     |     500 |                      +8.76% |              +31.39% |           0.365 -> 0.115 |            0 -> 0 |
| disabled     |    1000 |                     -24.40% |              +19.09% |           0.012 -> 0.405 |            0 -> 0 |
| global_mutex |     100 |                      -1.54% |               -2.93% |           0.186 -> 0.155 |            0 -> 0 |
| global_mutex |     500 |                      +1.86% |              +94.66% |           0.306 -> 0.203 |            0 -> 0 |
| global_mutex |    1000 |                     +11.76% |               -0.27% |           0.173 -> 0.127 |            0 -> 0 |
| sharded      |     100 |                     +46.90% |              -55.24% |           0.442 -> 0.181 |            0 -> 0 |
| sharded      |     500 |                     +18.99% |              -53.73% |           0.536 -> 0.253 |            0 -> 0 |
| sharded      |    1000 |                     +13.29% |              -61.94% |           0.388 -> 0.087 |            0 -> 0 |
| thread_local |     100 |                     +62.25% |              -80.43% |           0.377 -> 0.148 |            0 -> 0 |
| thread_local |     500 |                     -53.33% |              +83.54% |           1.142 -> 0.983 |    15960 -> 15980 |
| thread_local |    1000 |    NA (4-core throughput=0) |    NA (4-core p99=0) |           0.000 -> 1.732 |    30000 -> 29980 |

### 4.5 Contention Measurements

Lock wait data from `cmdstat.txt` (global mutex only):

| Dataset                                    | Configuration       | cmdstat_lock_wait_us | Sum(total_time_us) | Estimated contention ratio | GET calls | SET calls |
| ------------------------------------------ | ------------------- | -------------------: | -----------------: | -------------------------: | --------: | --------: |
| final_matrix/20260408_154844               | 4-core global_mutex |                22291 |             364273 |                    6.1193% |     60418 |     59582 |
| final_matrix/20260408_154844               | 8-core global_mutex |                33634 |             424261 |                    7.9277% |     59750 |     60250 |
| metrics_strategy_mandatory/20260408_010639 | global_mutex        |                74304 |             764382 |                    9.7208% |     60040 |     59960 |

Lock-wait correlation entries (factual comparison to matrix cells):

- Final matrix global mutex lock_wait_us: `33634` (8-core) > `22291` (4-core).
- At c=500, throughput overhead vs disabled: `+14.89%` (8-core) vs `+9.13%` (4-core).
- At c=1000, throughput overhead vs disabled: `-21.00%` (8-core) vs `+18.15%` (4-core).

## 5. Observed Phenomena (STRICTLY FACTUAL)

- In the primary matrix, only `thread_local` has non-zero error counts, and only at `500` and `1000` clients.
- `thread_local` records:
  - 4-core: errors `15960` (c=500), `30000` (c=1000)
  - 8-core: errors `15980` (c=500), `29980` (c=1000)
- At 4-core and 1000 clients, `thread_local` aggregate throughput is `0.00` ops/sec.
- At 8-core and 1000 clients, `thread_local` aggregate throughput is `11.26` ops/sec with p99 `72800.00` us.
- For `sharded` at 4-core, p99 CV reaches `1.318` at c=1000.
- For `thread_local`, throughput CV reaches `1.142` (4-core c=500), `0.983` (8-core c=500), and `1.732` (8-core c=1000).
- In final matrix `global_mutex` cmdstat, lock wait is reported and non-zero for both core setups.
- In final matrix `disabled`, `global_mutex`, and `sharded`, total errors are `0` for all listed cells.
- Final matrix `thread_local` cmdstat files are empty (`0` bytes) for both core setups.

## 6. Data Integrity Notes

### 6.1 Run counts and completeness

- Primary matrix (`20260408_154844`) has:
  - machine metadata file
  - 8 strategy/core directories (`2 cores x 4 strategies`)
  - benchmark JSON for all 8 directories
  - `runs_per_config=3` in all primary JSON outputs
- In each successful per-run record, `total_ops` is `10000`.

### 6.2 Known anomalies and missing data

- Incomplete matrix attempt: `results/final_matrix/20260408_154750/`
  - Contains `machine_details.txt`
  - Contains `core_4/disabled/` directory with no files
- Supplementary `results/macos_m2/20260408_005543/summary.csv` contains header only (no rows).
- Supplementary `results/macos_m2/20260408_005543/core_2/thread_local/` has server log only (no benchmark JSON, no cmdstat).
- Supplementary `results/macos_m2/20260408_005543/core_2/sharded/cmdstat.txt` is empty.
- Primary matrix `thread_local` cmdstat files are empty.

### 6.3 Noise considerations

- Memory samples in benchmark JSON are zero on macOS (`rss_bytes=0`, `vsize_bytes=0`) because the benchmark memory reader uses `/proc/self/statm`.
- Server startup logs show AOF replay before each run set; loaded command counts increase across matrix steps (from `909263` to `1290703`).
- Thread-local failures in benchmark outputs are not accompanied by explicit panic/error lines in the corresponding server logs.
- High CV values appear in multiple cells (notably `sharded` and `thread_local`), indicating high run-to-run dispersion in those cells.

### 6.4 Supplementary datasets retained

Mandatory strategy run (`results/metrics_strategy_mandatory/20260408_010639`):

| Strategy     | Clients | Throughput mean | Throughput variance | p99 mean (us) | p99 variance | Errors |
| ------------ | ------: | --------------: | ------------------: | ------------: | -----------: | -----: |
| global_mutex |     100 |       128427.62 |        738483967.62 |       2481.67 |   1370964.33 |      0 |
| global_mutex |     500 |        85309.66 |        921349007.43 |      12181.67 |  93004830.33 |      0 |
| global_mutex |    1000 |        75028.62 |       1019132535.44 |      14862.33 | 127218090.33 |      0 |
| thread_local |     100 |       129197.41 |        422333492.36 |       1992.00 |    567777.00 |      0 |
| thread_local |     500 |        95492.27 |       1607847416.61 |       4622.00 |   1969617.00 |      0 |
| thread_local |    1000 |        94999.85 |        355005531.33 |      17454.67 | 332449576.33 |      0 |

Auxiliary core-2 run (`results/macos_m2/20260408_005543/core_2`):

| Strategy     | Runs | Clients | Throughput mean +- stddev | Throughput CV | p99 mean +- stddev (us) | p99 CV | Errors |
| ------------ | ---: | ------: | ------------------------: | ------------: | ----------------------: | -----: | -----: |
| global_mutex |    5 |     100 |     169753.67 +- 23878.50 |         0.141 |       1315.40 +- 967.02 |  0.735 |      0 |
| global_mutex |    5 |     500 |      157067.66 +- 5800.40 |         0.037 |       3623.80 +- 855.04 |  0.236 |      0 |
| global_mutex |    5 |    1000 |     124016.42 +- 14658.02 |         0.118 |      7048.40 +- 3139.39 |  0.445 |      0 |
| sharded      |    5 |     100 |     154861.94 +- 21999.54 |         0.142 |       1308.40 +- 675.66 |  0.516 |      0 |
| sharded      |    5 |     500 |     138343.53 +- 12743.57 |         0.092 |       3390.60 +- 913.73 |  0.269 |      0 |
| sharded      |    5 |    1000 |     102548.31 +- 29391.57 |         0.287 |    19267.20 +- 29601.22 |  1.536 |   2120 |
