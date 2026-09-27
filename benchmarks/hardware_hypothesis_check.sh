#!/usr/bin/env bash
# Diagnostic script for the three fast/slow-state hypotheses on a Linux
# i3-10110U / 8GB laptop:
#   A. Thermal throttling
#   B. Scheduler bouncing the server between performance/efficiency
#      contexts (this CPU has no E-cores, so the analogous risk here is
#      cpufreq governor / turbo-boost instability, not P/E-core scheduling)
#   C. Memory pressure (swap activity under load)
#
# This does not run the full benchmark matrix; it runs one sustained
# workload against an already-started RustRedis server and logs the
# machine-state signals needed to tell the hypotheses apart. Read the
# output CSV afterwards, or diff two runs (e.g. cool room vs warm room,
# taskset-pinned vs unpinned).
#
# Usage:
#   ./benchmarks/hardware_hypothesis_check.sh [output_csv] [duration_secs] [concurrency]
#
# Prerequisites: server already running on 127.0.0.1:6379, and
# benchmarks/target/release/rustredis-bench already built.

set -euo pipefail

OUT_CSV="${1:-hardware_diag.csv}"
DURATION_SECS="${2:-60}"
CONCURRENCY="${3:-300}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "elapsed_secs,cpu_freq_mean_mhz,cpu_freq_max_mhz,temp_max_c,load_1m,mem_available_kb,swap_free_kb,ac_online" > "$OUT_CSV"

echo "Starting sustained load: concurrency=$CONCURRENCY duration~=${DURATION_SECS}s"
echo "Logging machine state to $OUT_CSV every 1s"
echo
echo "Hypothesis A (thermal throttling): watch temp_max_c and cpu_freq_mean_mhz together."
echo "  If frequency drops as temperature rises past ~85-95C, throttling is active."
echo "  Rerun this script with a fan/cooling pad and compare the two CSVs."
echo
echo "Hypothesis B (frequency/turbo instability): watch cpu_freq_mean_mhz alone."
echo "  Large swings even without a temperature rise point at governor/turbo behavior,"
echo "  not heat. Try: sudo cpupower frequency-set -g performance (pins the governor)"
echo "  and rerun, or pin the server to specific cores with:"
echo "    taskset -c 0,1 ./target/release/server"
echo "  to see if pinning stabilizes throughput."
echo
echo "Hypothesis C (memory pressure): watch mem_available_kb and swap_free_kb."
echo "  On this 8GB machine, if mem_available_kb falls below ~500000 (500MB) or"
echo "  swap_free_kb drops noticeably during the run, memory pressure is a plausible cause."
echo

(
  "$ROOT_DIR/benchmarks/target/release/rustredis-bench" \
    --host 127.0.0.1 --port 6379 \
    --concurrency "$CONCURRENCY" \
    --requests 100000 \
    --runs 1 \
    --workload mixed \
    --key-space 10000 \
    --value-size 64 \
    --output-dir /tmp/hardware_diag_bench_out \
    > /tmp/hardware_diag_bench.log 2>&1
) &
BENCH_PID=$!

START=$(date +%s)
while kill -0 "$BENCH_PID" 2>/dev/null; do
  NOW=$(date +%s)
  ELAPSED=$((NOW - START))

  FREQS=()
  for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq; do
    [ -r "$f" ] && FREQS+=("$(cat "$f")")
  done
  if [ "${#FREQS[@]}" -gt 0 ]; then
    SUM=0
    MAX=0
    for v in "${FREQS[@]}"; do
      SUM=$((SUM + v))
      [ "$v" -gt "$MAX" ] && MAX=$v
    done
    FREQ_MEAN_MHZ=$(awk -v s="$SUM" -v n="${#FREQS[@]}" 'BEGIN{printf "%.1f", (s/n)/1000}')
    FREQ_MAX_MHZ=$(awk -v m="$MAX" 'BEGIN{printf "%.1f", m/1000}')
  else
    FREQ_MEAN_MHZ=""
    FREQ_MAX_MHZ=""
  fi

  TEMP_MAX_MC=0
  for t in /sys/class/thermal/thermal_zone*/temp; do
    [ -r "$t" ] || continue
    V=$(cat "$t")
    [ "$V" -gt "$TEMP_MAX_MC" ] && TEMP_MAX_MC=$V
  done
  TEMP_MAX_C=$(awk -v m="$TEMP_MAX_MC" 'BEGIN{printf "%.1f", m/1000}')

  LOAD_1M=$(cut -d' ' -f1 /proc/loadavg)
  MEM_AVAIL_KB=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
  SWAP_FREE_KB=$(awk '/SwapFree/{print $2}' /proc/meminfo)
  AC_ONLINE=$(cat /sys/class/power_supply/AC/online 2>/dev/null || echo "")

  echo "${ELAPSED},${FREQ_MEAN_MHZ},${FREQ_MAX_MHZ},${TEMP_MAX_C},${LOAD_1M},${MEM_AVAIL_KB},${SWAP_FREE_KB},${AC_ONLINE}" >> "$OUT_CSV"

  sleep 1
done

wait "$BENCH_PID" || true
echo "Done. Bench log: /tmp/hardware_diag_bench.log"
echo "Machine-state trace: $OUT_CSV"
