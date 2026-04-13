#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found in PATH"
  exit 1
fi

if ! command -v redis-cli >/dev/null 2>&1; then
  echo "redis-cli not found in PATH"
  exit 1
fi

RUNS="${RUNS:-10}"
REQUESTS="${REQUESTS:-10000}"
WORKLOAD="${WORKLOAD:-mixed}"
KEY_SPACE="${KEY_SPACE:-10000}"
VALUE_SIZE="${VALUE_SIZE:-64}"
PORT="${PORT:-6379}"
SERVER_STARTUP_SECS="${SERVER_STARTUP_SECS:-1}"
SERVER_STARTUP_TIMEOUT_SECS="${SERVER_STARTUP_TIMEOUT_SECS:-30}"
RUN_COOLDOWN_SECS="${RUN_COOLDOWN_SECS:-3}"
CONFIG_COOLDOWN_SECS="${CONFIG_COOLDOWN_SECS:-5}"
RUN_RETRY_LIMIT="${RUN_RETRY_LIMIT:-5}"
TOKIO_WORKER_THREADS_FIXED="${TOKIO_WORKER_THREADS_FIXED:-$(sysctl -n hw.logicalcpu 2>/dev/null || echo 8)}"

STRATEGIES=("disabled" "global_mutex" "sharded" "thread_local")
CONCURRENCIES=(100 500 1000)

if redis-cli -p "$PORT" PING >/dev/null 2>&1; then
  echo "Port $PORT already responds to PING. Stop existing server before running controlled experiment."
  exit 1
fi

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_BASE="results/final_experiment/${TIMESTAMP}"
mkdir -p "$OUT_BASE"

{
  echo "timestamp=$(date -Iseconds)"
  echo "host=$(hostname)"
  echo "os=$(sw_vers 2>/dev/null | tr '\n' ';' || true)"
  echo "kernel=$(uname -a)"
  echo "cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'Apple Silicon (machdep unavailable)')"
  echo "logicalcpu=$(sysctl -n hw.logicalcpu 2>/dev/null || true)"
  echo "physicalcpu=$(sysctl -n hw.physicalcpu 2>/dev/null || true)"
  echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
  echo "rustc=$(rustc --version)"
  echo "cargo=$(cargo --version)"
  echo "tokio_worker_threads_fixed=$TOKIO_WORKER_THREADS_FIXED"
  echo "runs=$RUNS"
  echo "requests=$REQUESTS"
  echo "workload=$WORKLOAD"
  echo "key_space=$KEY_SPACE"
  echo "value_size=$VALUE_SIZE"
  echo "port=$PORT"
  echo "run_cooldown_secs=$RUN_COOLDOWN_SECS"
  echo "config_cooldown_secs=$CONFIG_COOLDOWN_SECS"
  echo "run_retry_limit=$RUN_RETRY_LIMIT"
  echo "strategies=${STRATEGIES[*]}"
  echo "concurrency_levels=${CONCURRENCIES[*]}"
} > "$OUT_BASE/machine_details.txt"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

aggregate_config_results() {
  local cfg_dir="$1"
  local expected_runs="$2"

  python3 - "$cfg_dir" "$expected_runs" <<'PY'
import json
import math
import statistics
import sys
import time
from pathlib import Path

cfg_dir = Path(sys.argv[1])
expected_runs = int(sys.argv[2])

run_json_paths = sorted(
  cfg_dir.glob("run_*/benchmark_results.json"),
  key=lambda p: int(p.parent.name.split("_")[1]),
)

per_runs = []
name = "Mixed (50% GET / 50% SET)"
concurrency = 0
target = "RustRedis"

for path in run_json_paths:
  payload = json.loads(path.read_text(encoding="utf-8"))
  if not payload.get("results"):
    continue

  result = payload["results"][0]
  name = result.get("name", name)
  concurrency = int(result.get("concurrency", concurrency))
  target = result.get("target", target)

  result_runs = result.get("per_run", [])
  if result_runs:
    per_runs.append(result_runs[0])

if len(per_runs) != expected_runs:
  raise SystemExit(
    f"Expected {expected_runs} per-run entries in {cfg_dir}, found {len(per_runs)}"
  )

ops_vals = [float(r.get("ops_per_sec", 0.0)) for r in per_runs]
p50_vals = [float(r.get("p50_us", 0.0)) for r in per_runs]
p99_vals = [float(r.get("p99_us", 0.0)) for r in per_runs]
max_vals = [float(r.get("max_us", 0.0)) for r in per_runs]
total_errors = int(sum(int(r.get("errors", 0)) for r in per_runs))


def mean(values):
  return sum(values) / len(values) if values else 0.0


def stddev(values):
  if len(values) < 2:
    return 0.0
  return statistics.stdev(values)


aggregated = {
  "name": name,
  "concurrency": concurrency,
  "target": target,
  "runs": len(per_runs),
  "ops_per_sec_mean": mean(ops_vals),
  "ops_per_sec_stddev": stddev(ops_vals),
  "p50_us_mean": mean(p50_vals),
  "p50_us_stddev": stddev(p50_vals),
  "p99_us_mean": mean(p99_vals),
  "p99_us_stddev": stddev(p99_vals),
  "max_us_mean": mean(max_vals),
  "max_us_stddev": stddev(max_vals),
  "total_errors": total_errors,
  "per_run": per_runs,
}

suite = {
  "timestamp": str(int(time.time())),
  "runs_per_config": len(per_runs),
  "results": [aggregated],
  "memory_samples": [],
}

(cfg_dir / "benchmark_results.json").write_text(
  json.dumps(suite, indent=2) + "\n", encoding="utf-8"
)
PY
}

echo "Building release binaries once..."
cargo build --release --bin server >/dev/null
cargo build --release --manifest-path benchmarks/Cargo.toml >/dev/null

echo "Running controlled experiment matrix..."
for strategy in "${STRATEGIES[@]}"; do
  for conc in "${CONCURRENCIES[@]}"; do
    cfg_dir="$OUT_BASE/${strategy}/c${conc}"
    mkdir -p "$cfg_dir"
    : > "$cfg_dir/bench_stdout.log"

    echo "=== strategy=${strategy} clients=${conc} ==="

    TOKIO_WORKER_THREADS="$TOKIO_WORKER_THREADS_FIXED" \
    RUSTREDIS_METRICS_STRATEGY="$strategy" \
    ./target/release/server >"$cfg_dir/server.log" 2>&1 &
    SERVER_PID=$!

    sleep "$SERVER_STARTUP_SECS"

    ready=0
    start_ts="$(date +%s)"
    while true; do
      if redis-cli -p "$PORT" PING >/dev/null 2>&1; then
        ready=1
        break
      fi

      now_ts="$(date +%s)"
      if (( now_ts - start_ts >= SERVER_STARTUP_TIMEOUT_SECS )); then
        break
      fi
      sleep 1
    done

    if [[ "$ready" -ne 1 ]]; then
      echo "Server failed to become ready for strategy=${strategy} clients=${conc}"
      exit 1
    fi

    for run_idx in $(seq 1 "$RUNS"); do
      run_dir="$cfg_dir/run_${run_idx}"
      mkdir -p "$run_dir"

      attempt=1
      while true; do
        if ./target/release/rustredis-bench \
          --host 127.0.0.1 \
          --port "$PORT" \
          --concurrency "$conc" \
          --requests "$REQUESTS" \
          --runs 1 \
          --workload "$WORKLOAD" \
          --key-space "$KEY_SPACE" \
          --value-size "$VALUE_SIZE" \
          --output-dir "$run_dir" \
          >"$run_dir/bench_stdout.log" 2>&1; then
          break
        fi

        if (( attempt >= RUN_RETRY_LIMIT )); then
          echo "Run failed after retries: strategy=${strategy} clients=${conc} run=${run_idx}"
          exit 1
        fi

        attempt=$((attempt + 1))
        sleep "$RUN_COOLDOWN_SECS"
      done

      {
        echo "===== run_${run_idx} ====="
        cat "$run_dir/bench_stdout.log"
        echo
      } >> "$cfg_dir/bench_stdout.log"

      sleep "$RUN_COOLDOWN_SECS"
    done

    aggregate_config_results "$cfg_dir" "$RUNS"

    redis-cli -p "$PORT" CMDSTAT > "$cfg_dir/cmdstat.txt" || true

    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
    unset SERVER_PID

    sleep "$CONFIG_COOLDOWN_SECS"
  done
done

mkdir -p results/final_experiment
echo "$OUT_BASE" > results/final_experiment/latest_run.txt

echo "OUT_DIR=$OUT_BASE"