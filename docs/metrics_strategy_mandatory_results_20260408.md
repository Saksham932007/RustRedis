# Metrics Strategy Mandatory Runs (2026-04-08)

## Step 1 (Mandatory): Executed Matrix

- Strategies: `GlobalMutex`, `ThreadLocal`
- Client configurations: `100`, `500`, `1000`
- Workload: `mixed` (50% GET / 50% SET)
- Runs per configuration: `3`
- Raw artifacts:
  - `results/metrics_strategy_mandatory/20260408_010639/global_mutex/benchmark_results.json`
  - `results/metrics_strategy_mandatory/20260408_010639/thread_local/benchmark_results.json`

## Step 2: Collected Metrics

| Strategy | Clients | Throughput mean (ops/sec) | Throughput variance | p99 mean (us) | p99 variance |
|---|---:|---:|---:|---:|---:|
| GlobalMutex | 100 | 128427.62 | 738483967.62 | 2481.67 | 1370964.33 |
| GlobalMutex | 500 | 85309.66 | 921349007.43 | 12181.67 | 93004830.33 |
| GlobalMutex | 1000 | 75028.62 | 1019132535.44 | 14862.33 | 127218090.33 |
| ThreadLocal | 100 | 129197.41 | 422333492.36 | 1992.00 | 567777.00 |
| ThreadLocal | 500 | 95492.27 | 1607847416.61 | 4622.00 | 1969617.00 |
| ThreadLocal | 1000 | 94999.85 | 355005531.33 | 17454.67 | 332449576.33 |

Variance formula used: `variance = (stddev)^2`.

## Step 3: Results To Send

- At `100` clients:
  - GlobalMutex: throughput `128427.62` ops/sec, p99 `2481.67` us, throughput variance `738483967.62`, p99 variance `1370964.33`
  - ThreadLocal: throughput `129197.41` ops/sec, p99 `1992.00` us, throughput variance `422333492.36`, p99 variance `567777.00`
- At `500` clients:
  - GlobalMutex: throughput `85309.66` ops/sec, p99 `12181.67` us, throughput variance `921349007.43`, p99 variance `93004830.33`
  - ThreadLocal: throughput `95492.27` ops/sec, p99 `4622.00` us, throughput variance `1607847416.61`, p99 variance `1969617.00`
- At `1000` clients:
  - GlobalMutex: throughput `75028.62` ops/sec, p99 `14862.33` us, throughput variance `1019132535.44`, p99 variance `127218090.33`
  - ThreadLocal: throughput `94999.85` ops/sec, p99 `17454.67` us, throughput variance `355005531.33`, p99 variance `332449576.33`

## Step 4 (User-owned)

- Write Results section
- Define final claims
- Structure full paper
