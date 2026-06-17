#!/usr/bin/env bash
set -euo pipefail

# Research benchmark runner for Apple Silicon Macs.
# Runs RustRedis across 2, 4, and 8 Tokio worker threads and collects
# results for global_mutex, sharded, and thread_local at 100/500/1000 clients.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found in PATH"
  exit 1
fi

if ! command -v redis-cli >/dev/null 2>&1; then
  echo "redis-cli not found. Install with: brew install redis"
  exit 1
fi

REQUESTS_PER_CLIENT="${REQUESTS_PER_CLIENT:-10000}"
RUNS="${RUNS:-5}"
WORKLOAD="${WORKLOAD:-mixed}"
KEY_SPACE="${KEY_SPACE:-10000}"
VALUE_SIZE="${VALUE_SIZE:-64}"
PORT="${PORT:-6379}"
SERVER_STARTUP_SECS="${SERVER_STARTUP_SECS:-3}"

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_BASE="results/macos_m2/${TIMESTAMP}"
mkdir -p "$OUT_BASE"

echo "Output directory: $OUT_BASE"

# Capture machine/environment details once for reproducibility.
{
  echo "timestamp=$(date -Iseconds)"
  echo "hostname=$(hostname)"
  echo "os=$(sw_vers 2>/dev/null | tr '\n' ';' || true)"
  echo "kernel=$(uname -a)"
  echo "cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'Apple Silicon (machdep unavailable)')"
  echo "logicalcpu=$(sysctl -n hw.logicalcpu 2>/dev/null || true)"
  echo "physicalcpu=$(sysctl -n hw.physicalcpu 2>/dev/null || true)"
  echo "mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || true)"
  echo "rustc=$(rustc --version)"
  echo "cargo=$(cargo --version)"
} > "$OUT_BASE/machine_details.txt"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

run_once() {
  local cores="$1"
  local strategy="$2"
  local run_dir="$OUT_BASE/core_${cores}/${strategy}"

  mkdir -p "$run_dir"

  echo
  echo "=== core=${cores} strategy=${strategy} ==="

  TOKIO_WORKER_THREADS="$cores" \
  RUSTREDIS_METRICS_STRATEGY="$strategy" \
  cargo run --release --bin server >"$run_dir/server.log" 2>&1 &
  SERVER_PID=$!

  sleep "$SERVER_STARTUP_SECS"

  if ! redis-cli -p "$PORT" PING >/dev/null 2>&1; then
    echo "server did not become ready; see $run_dir/server.log"
    exit 1
  fi

  cargo run --release --manifest-path benchmarks/Cargo.toml -- \
    --host 127.0.0.1 \
    --port "$PORT" \
    --concurrency "100,500,1000" \
    --requests "$REQUESTS_PER_CLIENT" \
    --runs "$RUNS" \
    --workload "$WORKLOAD" \
    --key-space "$KEY_SPACE" \
    --value-size "$VALUE_SIZE" \
    --metrics-strategy "$strategy" \
    --output-dir "$run_dir" \
    >"$run_dir/bench_stdout.log" 2>&1

  redis-cli -p "$PORT" CMDSTAT > "$run_dir/cmdstat.txt" || true

  # Ensure server is cleanly restarted between runs/configurations.
  kill "$SERVER_PID" >/dev/null 2>&1 || true
  wait "$SERVER_PID" 2>/dev/null || true
  unset SERVER_PID

  echo "Saved: $run_dir"
}

for cores in 2 4 8; do
  for strategy in global_mutex sharded thread_local; do
    run_once "$cores" "$strategy"
  done
done

echo
cat <<MSG
Done.

Raw data layout:
  $OUT_BASE/core_<2|4|8>/<strategy>/benchmark_results.json
  $OUT_BASE/core_<2|4|8>/<strategy>/cmdstat.txt

Next step:
  python3 benchmarks/summarize_macos_m2_results.py --input "$OUT_BASE" --output "$OUT_BASE/summary.csv"
MSG
