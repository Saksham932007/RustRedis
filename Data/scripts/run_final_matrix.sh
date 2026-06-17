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

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_BASE="results/final_matrix/${TIMESTAMP}"
mkdir -p "$OUT_BASE"

RUNS="${RUNS:-3}"
REQUESTS_PER_CLIENT="${REQUESTS_PER_CLIENT:-10000}"
WORKLOAD="${WORKLOAD:-mixed}"
CONCURRENCY="${CONCURRENCY:-100,500,1000}"
PORT="${PORT:-6379}"
SERVER_STARTUP_SECS="${SERVER_STARTUP_SECS:-3}"
CORES="${CORES:-4 8}"
STRATEGIES="${STRATEGIES:-disabled global_mutex sharded thread_local}"

{
  echo "timestamp=$(date -Iseconds)"
  echo "host=$(hostname)"
  echo "os=$(sw_vers 2>/dev/null | tr '\n' ';' || true)"
  echo "kernel=$(uname -a)"
  echo "logicalcpu=$(sysctl -n hw.logicalcpu 2>/dev/null || true)"
  echo "physicalcpu=$(sysctl -n hw.physicalcpu 2>/dev/null || true)"
  echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
  echo "rustc=$(rustc --version)"
  echo "cargo=$(cargo --version)"
  echo "runs=$RUNS"
  echo "requests_per_client=$REQUESTS_PER_CLIENT"
  echo "workload=$WORKLOAD"
  echo "concurrency=$CONCURRENCY"
  echo "cores=$CORES"
  echo "strategies=$STRATEGIES"
} > "$OUT_BASE/machine_details.txt"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

run_once() {
  local core="$1"
  local strategy="$2"
  local out_dir="$OUT_BASE/core_${core}/${strategy}"
  mkdir -p "$out_dir"

  echo "=== core=${core} strategy=${strategy} ==="

  TOKIO_WORKER_THREADS="$core" \
  RUSTREDIS_METRICS_STRATEGY="$strategy" \
  cargo run --release --bin server >"$out_dir/server.log" 2>&1 &
  SERVER_PID=$!

  sleep "$SERVER_STARTUP_SECS"

  if ! redis-cli -p "$PORT" PING >/dev/null 2>&1; then
    echo "server failed to start for core=${core} strategy=${strategy}" | tee -a "$out_dir/bench_stdout.log"
    return 1
  fi

  cargo run --release --manifest-path benchmarks/Cargo.toml -- \
    --host 127.0.0.1 \
    --port "$PORT" \
    --concurrency "$CONCURRENCY" \
    --requests "$REQUESTS_PER_CLIENT" \
    --runs "$RUNS" \
    --workload "$WORKLOAD" \
    --key-space 10000 \
    --value-size 64 \
    --metrics-strategy "$strategy" \
    --output-dir "$out_dir" \
    >"$out_dir/bench_stdout.log" 2>&1

  redis-cli -p "$PORT" CMDSTAT > "$out_dir/cmdstat.txt" || true

  kill "$SERVER_PID" >/dev/null 2>&1 || true
  wait "$SERVER_PID" 2>/dev/null || true
  unset SERVER_PID
}

for core in $CORES; do
  for strategy in $STRATEGIES; do
    run_once "$core" "$strategy"
  done
done

echo "OUT_DIR=$OUT_BASE"
