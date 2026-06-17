# final_experiment_summary

Generated at: 2026-04-21T17:10:18Z

## Artifact Paths

- JSON: final_experiment_v12.json
- Raw matrix source: experiment_results_v12/raw_data.csv
- Metadata source: experiment_results_v12/metadata.json
- System validation source: experiment_results_v12/system_validation/20260421_170502
- Config file: experiment_results_v12/final_experiment_config.json

## Matrix Coverage

- Strategies: Disabled, GlobalMutex, Sharded-2key, ThreadLocal, HdrHistogram, Sharded-N
- Concurrency levels: 100, 200, 300, 400, 500, 600, 700, 1000
- Runs per configuration: 30
- Warmup/measured per client: 100 / 900
- Workload: 50% GET / 50% SET
- Keyspace/value size: 10,000 / 64 bytes

## Section Checklist

- Section 1 raw configs: 48
- Section 2 aggregated configs: 48
- Section 3 statistical tests: 18
- Section 4 Shapiro-Wilk rows: 48
- Section 5 system insights: included
- Section 6 Table 5.4.3 rows: 18
- Section 7 reproducibility metadata: included

## Primary Comparisons (500/600/700; throughput + p99)

| Comparison | Concurrency | Metric | Test | p-value | Effect |
|---|---:|---|---|---:|---:|
| Disabled vs Sharded-2key | 500 | throughput | Mann-Whitney U | 0.0251013 | 0.337778 |
| Disabled vs Sharded-2key | 500 | p99 | Mann-Whitney U | 0.678896 | 0.0633333 |
| Disabled vs HdrHistogram | 500 | throughput | Mann-Whitney U | 1.20567e-10 | 0.968889 |
| Disabled vs HdrHistogram | 500 | p99 | Mann-Whitney U | 2.60985e-10 | -0.951111 |
| Sharded-2key vs GlobalMutex | 500 | throughput | Mann-Whitney U | 3.01986e-11 | 1 |
| Sharded-2key vs GlobalMutex | 500 | p99 | Mann-Whitney U | 3.01986e-11 | -1 |
| Disabled vs Sharded-2key | 600 | throughput | Mann-Whitney U | 0.0098834 | 0.388889 |
| Disabled vs Sharded-2key | 600 | p99 | Mann-Whitney U | 0.657376 | -0.0677778 |
| Disabled vs HdrHistogram | 600 | throughput | Mann-Whitney U | 3.01986e-11 | 1 |
| Disabled vs HdrHistogram | 600 | p99 | Mann-Whitney U | 3.01986e-11 | -1 |
| Sharded-2key vs GlobalMutex | 600 | throughput | Mann-Whitney U | 3.01986e-11 | 1 |
| Sharded-2key vs GlobalMutex | 600 | p99 | Mann-Whitney U | 3.01986e-11 | -1 |
| Disabled vs Sharded-2key | 700 | throughput | Mann-Whitney U | 1.24932e-05 | 0.657778 |
| Disabled vs Sharded-2key | 700 | p99 | Mann-Whitney U | 0.00318296 | -0.444444 |
| Disabled vs HdrHistogram | 700 | throughput | Mann-Whitney U | 3.01986e-11 | 1 |
| Disabled vs HdrHistogram | 700 | p99 | Welch t-test | 9.22979e-27 | -9.80645 |
| Sharded-2key vs GlobalMutex | 700 | throughput | Mann-Whitney U | 3.01986e-11 | 1 |
| Sharded-2key vs GlobalMutex | 700 | p99 | Mann-Whitney U | 3.01986e-11 | -1 |

## Shapiro-Wilk (400/500/600/700)

| Strategy | Concurrency | Metric | W | p-value | Normal (p>=0.05) |
|---|---:|---|---:|---:|---|
| Disabled | 400 | throughput | 0.914261 | 0.0190877 | False |
| Disabled | 400 | p99 | 0.93252 | 0.0573051 | True |
| Disabled | 500 | throughput | 0.857365 | 0.000887811 | False |
| Disabled | 500 | p99 | 0.848969 | 0.000590014 | False |
| Disabled | 600 | throughput | 0.752007 | 9.81437e-06 | False |
| Disabled | 600 | p99 | 0.809819 | 9.95124e-05 | False |
| Disabled | 700 | throughput | 0.828646 | 0.000228551 | False |
| Disabled | 700 | p99 | 0.938242 | 0.0815719 | True |
| GlobalMutex | 400 | throughput | 0.685581 | 9.68946e-07 | False |
| GlobalMutex | 400 | p99 | 0.807548 | 9.02696e-05 | False |
| GlobalMutex | 500 | throughput | 0.949114 | 0.160057 | True |
| GlobalMutex | 500 | p99 | 0.837826 | 0.000348377 | False |
| GlobalMutex | 600 | throughput | 0.983997 | 0.918897 | True |
| GlobalMutex | 600 | p99 | 0.967499 | 0.473194 | True |
| GlobalMutex | 700 | throughput | 0.961198 | 0.332278 | True |
| GlobalMutex | 700 | p99 | 0.966203 | 0.441135 | True |
| Sharded-2key | 400 | throughput | 0.874745 | 0.00214014 | False |
| Sharded-2key | 400 | p99 | 0.965249 | 0.418553 | True |
| Sharded-2key | 500 | throughput | 0.893472 | 0.0058366 | False |
| Sharded-2key | 500 | p99 | 0.926452 | 0.0395565 | False |
| Sharded-2key | 600 | throughput | 0.455156 | 1.85565e-09 | False |
| Sharded-2key | 600 | p99 | 0.688061 | 1.05068e-06 | False |
| Sharded-2key | 700 | throughput | 0.772009 | 2.10905e-05 | False |
| Sharded-2key | 700 | p99 | 0.814446 | 0.000121595 | False |
| ThreadLocal | 400 | throughput | 0.92141 | 0.029182 | False |
| ThreadLocal | 400 | p99 | 0.976506 | 0.727006 | True |
| ThreadLocal | 500 | throughput | 0.916396 | 0.0216482 | False |
| ThreadLocal | 500 | p99 | 0.695771 | 1.35487e-06 | False |
| ThreadLocal | 600 | throughput | 0.818327 | 0.000144134 | False |
| ThreadLocal | 600 | p99 | 0.952709 | 0.199691 | True |
| ThreadLocal | 700 | throughput | 0.920615 | 0.0278261 | False |
| ThreadLocal | 700 | p99 | 0.912286 | 0.0170008 | False |
| HdrHistogram | 400 | throughput | 0.771113 | 2.03647e-05 | False |
| HdrHistogram | 400 | p99 | 0.575324 | 3.69644e-08 | False |
| HdrHistogram | 500 | throughput | 0.703983 | 1.78384e-06 | False |
| HdrHistogram | 500 | p99 | 0.848853 | 0.000586725 | False |
| HdrHistogram | 600 | throughput | 0.866461 | 0.0013988 | False |
| HdrHistogram | 600 | p99 | 0.944692 | 0.121702 | True |
| HdrHistogram | 700 | throughput | 0.875048 | 0.00217418 | False |
| HdrHistogram | 700 | p99 | 0.966927 | 0.458877 | True |
| Sharded-N | 400 | throughput | 0.674413 | 6.75952e-07 | False |
| Sharded-N | 400 | p99 | 0.805379 | 8.22883e-05 | False |
| Sharded-N | 500 | throughput | 0.83384 | 0.000289724 | False |
| Sharded-N | 500 | p99 | 0.978564 | 0.786209 | True |
| Sharded-N | 600 | throughput | 0.798432 | 6.13933e-05 | False |
| Sharded-N | 600 | p99 | 0.931565 | 0.0540432 | True |
| Sharded-N | 700 | throughput | 0.915359 | 0.020362 | False |
| Sharded-N | 700 | p99 | 0.973044 | 0.625366 | True |
