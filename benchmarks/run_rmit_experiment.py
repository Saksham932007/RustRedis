#!/usr/bin/env python3
"""RMIT (Randomized Multiple Interleaved Trials) experiment runner.

The v12 runner (`run_final_experiment_v12.py`) finishes all 30 runs of one
strategy/concurrency configuration, then all runs of the next, in a fixed
order. That confounds strategy identity with time-of-day / thermal state:
if the machine transitions between a "fast" and "slow" state partway
through the matrix, whole strategies end up biased rather than individual
runs. This runner fixes that by drawing a fresh random permutation of all
(strategy, concurrency) pairs for every repetition and executing that
repetition's runs in the shuffled order, restarting the server between
every single run (order is no longer strategy-grouped, so every run is a
strategy switch anyway). Every run's machine state (CPU frequency,
temperature, memory/swap, load average, AC/battery) is captured
immediately before it executes, so fast/slow states can be explained
afterwards instead of only observed.

This machine is an Intel i3-10110U (2 cores / 4 threads) with 8GB RAM, not
the 8-core M2 the v12/macOS protocol was tuned for. Defaults below
(concurrency ceiling, repetitions) are scaled down accordingly; override
with flags if you run this on stronger hardware.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import platform
import random
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent))
import system_state  # noqa: E402


@dataclass(frozen=True)
class StrategySpec:
    key: str
    env_value: str
    label: str


STRATEGIES: List[StrategySpec] = [
    StrategySpec("disabled", "disabled", "Disabled"),
    StrategySpec("global_mutex", "global_mutex", "GlobalMutex"),
    StrategySpec("sharded_2key", "sharded_2key", "Sharded-2key"),
    StrategySpec("thread_local", "thread_local", "ThreadLocal"),
    StrategySpec("hdr_histogram", "hdr_histogram", "HdrHistogram"),
    StrategySpec("sharded_n", "sharded_n", "Sharded-N"),
]
STRATEGY_BY_KEY = {s.key: s for s in STRATEGIES}

# Scaled down from the v12 macOS matrix (100..1000, 30 reps) for a 2c/4t,
# 8GB laptop. Override with --concurrency / --runs for stronger hardware.
DEFAULT_CONCURRENCY_LEVELS = [25, 50, 100, 150, 200, 300, 400, 500]
DEFAULT_REPS = 15
DEFAULT_WORKLOADS = ["mixed"]
VALID_WORKLOADS = ("mixed", "read-heavy", "write-heavy")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Randomized interleaved trials (RMIT) experiment runner")
    p.add_argument("--output-dir", default="experiment_results_rmit", help="Output directory")
    p.add_argument("--runs", type=int, default=DEFAULT_REPS, help="Repetitions per configuration")
    p.add_argument("--requests-per-client", type=int, default=1000)
    p.add_argument("--key-space", type=int, default=10000)
    p.add_argument("--value-size", type=int, default=64)
    p.add_argument(
        "--concurrency",
        default=",".join(str(c) for c in DEFAULT_CONCURRENCY_LEVELS),
        help="Comma-separated concurrency levels",
    )
    p.add_argument(
        "--workloads",
        default=",".join(DEFAULT_WORKLOADS),
        help=f"Comma-separated workload types, any of {VALID_WORKLOADS}. "
             "Adding more than one makes this a 3-D design: strategy x concurrency x workload, "
             "shuffled together per repetition.",
    )
    p.add_argument("--inter-run-cooldown-secs", type=float, default=2.0)
    p.add_argument("--run-retry-limit", type=int, default=3)
    p.add_argument("--port", type=int, default=6379)
    p.add_argument("--worker-threads", type=int, default=4, help="Tokio worker threads (this CPU has 4)")
    p.add_argument("--server-startup-timeout-secs", type=int, default=60)
    p.add_argument("--skip-build", action="store_true")
    p.add_argument("--seed", type=int, default=None, help="RNG seed for reproducible shuffles")
    p.add_argument(
        "--resume-run-id",
        default=None,
        help="Resume into an existing run_id directory instead of starting a fresh one",
    )
    return p.parse_args()


def run_text(cmd: Sequence[str], cwd: Path, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(list(cmd), cwd=cwd, capture_output=True, text=True, check=check)


def redis_ping(root_dir: Path, port: int) -> bool:
    """Pure-Python RESP PING — avoids depending on redis-cli being installed."""
    import socket

    try:
        with socket.create_connection(("127.0.0.1", port), timeout=1.0) as sock:
            sock.sendall(b"*1\r\n$4\r\nPING\r\n")
            data = sock.recv(4096)
            return b"PONG" in data
    except OSError:
        return False


def wait_for_server(root_dir: Path, port: int, proc: subprocess.Popen, timeout_secs: int) -> None:
    start = time.time()
    while True:
        if redis_ping(root_dir, port):
            return
        if proc.poll() is not None:
            raise RuntimeError(f"Server exited early with code {proc.returncode}")
        if time.time() - start >= timeout_secs:
            raise TimeoutError(f"Server not ready on port {port} within {timeout_secs}s")
        time.sleep(0.3)


def stop_server(proc: subprocess.Popen, log_handle) -> None:
    try:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=10)
    finally:
        log_handle.close()


def build_binaries(root_dir: Path) -> None:
    print("Building release binaries...")
    run_text(["cargo", "build", "--release", "--bin", "server"], cwd=root_dir)
    run_text(["cargo", "build", "--release", "--manifest-path", "benchmarks/Cargo.toml"], cwd=root_dir)


def start_server(
    root_dir: Path,
    strategy: StrategySpec,
    worker_threads: int,
    port: int,
    timeout_secs: int,
    log_path: Path,
) -> Tuple[subprocess.Popen, object]:
    env = os.environ.copy()
    env["TOKIO_WORKER_THREADS"] = str(worker_threads)
    env["RUSTREDIS_METRICS_STRATEGY"] = strategy.env_value
    env["RUSTREDIS_DISABLE_AOF"] = "1"

    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_handle = log_path.open("w", encoding="utf-8")
    proc = subprocess.Popen(
        ["./target/release/server"],
        cwd=root_dir,
        env=env,
        stdout=log_handle,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        wait_for_server(root_dir, port, proc, timeout_secs)
    except Exception:
        stop_server(proc, log_handle)
        raise
    return proc, log_handle


def run_bench_once(
    root_dir: Path,
    port: int,
    concurrency: int,
    requests_per_client: int,
    key_space: int,
    value_size: int,
    workload: str,
    run_dir: Path,
) -> int:
    run_dir.mkdir(parents=True, exist_ok=True)
    command = [
        "./target/release/rustredis-bench",
        "--host", "127.0.0.1",
        "--port", str(port),
        "--concurrency", str(concurrency),
        "--requests", str(requests_per_client),
        "--runs", "1",
        "--workload", workload,
        "--key-space", str(key_space),
        "--value-size", str(value_size),
        "--output-dir", str(run_dir),
    ]
    log_path = run_dir / "bench_stdout.log"
    with log_path.open("w", encoding="utf-8") as fh:
        proc = subprocess.run(command, cwd=root_dir, stdout=fh, stderr=subprocess.STDOUT, text=True)
    return proc.returncode


def parse_benchmark_run(run_json_path: Path) -> Dict[str, float]:
    payload = json.loads(run_json_path.read_text(encoding="utf-8"))
    if not payload.get("results"):
        raise ValueError(f"No results in {run_json_path}")
    result = payload["results"][0]
    per_run = result.get("per_run", [])
    row = per_run[0] if per_run else result
    return {
        "throughput": float(row.get("ops_per_sec", 0.0)),
        "p50": float(row.get("p50_us", 0.0)),
        "p99": float(row.get("p99_us", 0.0)),
        "avg_latency": float(row.get("avg_us", 0.0)),
        "latency_stddev": float(row.get("latency_stddev_us", 0.0)),
        "latency_cv": float(row.get("latency_cv", 0.0)),
        "errors": int(row.get("errors", 0)),
        "warmup_ops_per_client": int(row.get("warmup_ops_per_client", 0)),
        "measured_ops_per_client": int(row.get("measured_ops_per_client", 0)),
    }


def collect_machine_specs(root_dir: Path) -> Dict[str, object]:
    def val(cmd: Sequence[str]) -> str:
        r = run_text(cmd, cwd=root_dir, check=False)
        return (r.stdout or "").strip()

    def read_int(path: str) -> Optional[int]:
        try:
            return int(Path(path).read_text().strip())
        except (OSError, ValueError):
            return None

    processor = ""
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                processor = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass

    mem_total_kb = None
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                mem_total_kb = int(line.split()[1])
                break
    except OSError:
        pass

    return {
        # Real hostname is intentionally never recorded here (would leak into
        # committed public artifacts); this identifies *which environment*
        # ran the benchmark without naming the machine.
        "environment": os.environ.get("RMIT_ENVIRONMENT_LABEL", "unlabeled"),
        "platform": platform.platform(),
        "processor": processor or "unknown",
        "python_version": platform.python_version(),
        "logical_cpus": os.cpu_count(),
        "mem_total_bytes": (mem_total_kb * 1024) if mem_total_kb else None,
        "rustc": val(["rustc", "--version"]),
        "cargo": val(["cargo", "--version"]),
        "commit_hash": val(["git", "rev-parse", "HEAD"]),
    }


def raise_fd_limit(min_soft_limit: int = 65536) -> None:
    """Raise this process's open-file-descriptor soft limit as high as the
    hard limit allows. Server and benchmark-client subprocesses inherit
    rlimits from their parent, so doing this once here covers both without
    touching /etc/security/limits.conf or needing root. Without this, the
    default soft limit (often 1024) silently caps how many concurrent
    client connections can actually open sockets — every run above that
    ceiling reports 0 ops/sec with no error message, not a clean failure."""
    try:
        import resource
    except ImportError:
        return  # not available on this platform; caller should check ulimit -n manually

    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    target = min(min_soft_limit, hard)
    if soft < target:
        resource.setrlimit(resource.RLIMIT_NOFILE, (target, hard))


def main() -> None:
    args = parse_args()
    raise_fd_limit()
    root_dir = Path(__file__).resolve().parents[1]
    output_dir = (root_dir / args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    if redis_ping(root_dir, args.port):
        raise SystemExit(f"Port {args.port} already responds to PING. Stop the existing server first.")

    if not args.skip_build:
        build_binaries(root_dir)

    concurrency_levels = [int(c.strip()) for c in args.concurrency.split(",") if c.strip()]
    workloads = [w.strip() for w in args.workloads.split(",") if w.strip()]
    for w in workloads:
        if w not in VALID_WORKLOADS:
            raise SystemExit(f"Invalid workload '{w}'; must be one of {VALID_WORKLOADS}")

    try:
        import resource
        soft_fd_limit, _ = resource.getrlimit(resource.RLIMIT_NOFILE)
        needed = max(concurrency_levels) + 256  # client sockets + stdio/log/misc headroom
        if soft_fd_limit < needed:
            print(f"WARNING: open-file-descriptor soft limit is {soft_fd_limit}, but the highest "
                  f"concurrency level ({max(concurrency_levels)}) needs at least ~{needed}. "
                  f"raise_fd_limit() could not raise it far enough (hard limit too low) — runs at "
                  f"high concurrency will silently report 0 ops/sec. Check `ulimit -Hn` and your "
                  f"container/systemd/user-session fd limits.")
    except ImportError:
        pass

    rng = random.Random(args.seed)

    run_id = args.resume_run_id or datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    run_data_root = output_dir / "run_data" / run_id
    run_data_root.mkdir(parents=True, exist_ok=True)

    raw_rows_path = output_dir / "raw_data_rmit.csv"
    fieldnames = [
        "block_id", "order_in_block", "strategy", "strategy_label", "concurrency", "workload", "run_id",
        "throughput", "p50", "p99", "avg_latency", "latency_stddev", "latency_cv", "errors",
        "warmup_ops_per_client", "measured_ops_per_client",
    ] + system_state.STATE_FIELDNAMES

    # Resume support: if the CSV already exists, keep appending and skip
    # (block, strategy, concurrency, workload) combos already recorded.
    already_done: set = set()
    write_header = not raw_rows_path.exists()
    if raw_rows_path.exists():
        with raw_rows_path.open("r", newline="", encoding="utf-8") as fh:
            for row in csv.DictReader(fh):
                already_done.add((
                    int(row["block_id"]), row["strategy"], int(row["concurrency"]),
                    row.get("workload", "mixed"),
                ))

    total_blocks = args.runs
    total_configs = len(STRATEGIES) * len(concurrency_levels) * len(workloads)
    print(f"RMIT runner: {len(STRATEGIES)} strategies x {len(concurrency_levels)} concurrency levels "
          f"x {len(workloads)} workloads x {total_blocks} repetitions = {total_configs * total_blocks} runs")
    print(f"Concurrency levels: {concurrency_levels}")
    print(f"Workloads: {workloads}")
    print(f"Output: {output_dir}")

    csv_file = raw_rows_path.open("a", newline="", encoding="utf-8")
    writer = csv.DictWriter(csv_file, fieldnames=fieldnames)
    if write_header:
        writer.writeheader()
        csv_file.flush()

    try:
        for block_id in range(1, total_blocks + 1):
            triples: List[Tuple[StrategySpec, int, str]] = [
                (s, c, w) for s in STRATEGIES for c in concurrency_levels for w in workloads
            ]
            rng.shuffle(triples)  # fresh random order every repetition (RMIT core requirement)

            print(f"\n=== Block {block_id}/{total_blocks} (shuffled order of {len(triples)} configs) ===")

            for order_in_block, (strategy, concurrency, workload) in enumerate(triples, start=1):
                key = (block_id, strategy.key, concurrency, workload)
                if key in already_done:
                    print(f"  [{order_in_block:>3}/{len(triples)}] {strategy.key}/c{concurrency}/{workload} "
                          f"(block {block_id}) — already recorded, skipping")
                    continue

                cfg_dir = run_data_root / f"block{block_id}" / f"{strategy.key}_c{concurrency}_{workload}"

                state = system_state.snapshot()

                server_proc, log_handle = start_server(
                    root_dir=root_dir,
                    strategy=strategy,
                    worker_threads=args.worker_threads,
                    port=args.port,
                    timeout_secs=args.server_startup_timeout_secs,
                    log_path=cfg_dir / "server.log",
                )

                row: Optional[Dict[str, object]] = None
                last_error: Optional[str] = None
                try:
                    for attempt in range(1, args.run_retry_limit + 1):
                        run_dir = cfg_dir / f"attempt_{attempt}"
                        if run_dir.exists():
                            shutil.rmtree(run_dir)

                        rc = run_bench_once(
                            root_dir=root_dir,
                            port=args.port,
                            concurrency=concurrency,
                            requests_per_client=args.requests_per_client,
                            key_space=args.key_space,
                            value_size=args.value_size,
                            workload=workload,
                            run_dir=run_dir,
                        )
                        run_json = run_dir / "benchmark_results.json"
                        if rc == 0 and run_json.exists():
                            parsed = parse_benchmark_run(run_json)
                            row = {
                                "block_id": block_id,
                                "order_in_block": order_in_block,
                                "strategy": strategy.key,
                                "strategy_label": strategy.label,
                                "concurrency": concurrency,
                                "workload": workload,
                                "run_id": attempt,
                                **parsed,
                                **state,
                            }
                            break
                        last_error = f"block={block_id} {strategy.key}/c{concurrency}/{workload} attempt={attempt} rc={rc}"
                        time.sleep(args.inter_run_cooldown_secs)
                finally:
                    stop_server(server_proc, log_handle)

                if row is None:
                    raise RuntimeError(last_error or "benchmark run failed after retries")

                writer.writerow(row)
                csv_file.flush()

                print(
                    f"  [{order_in_block:>3}/{len(triples)}] {strategy.key:14s} c={concurrency:<4} {workload:12s} "
                    f"{row['throughput']:>9.0f} ops/sec  p99={row['p99']:>8.0f}us  "
                    f"cpu={state.get('cpu_freq_mean_mhz')}MHz temp={state.get('temp_max_celsius')}C "
                    f"load1={state.get('load_1m')} ac={state.get('ac_online')}"
                )

                time.sleep(args.inter_run_cooldown_secs)
    finally:
        csv_file.close()

    machine_specs = collect_machine_specs(root_dir)
    metadata = {
        "design": "RMIT (randomized multiple interleaved trials): fresh random permutation of "
                  "(strategy, concurrency, workload) triples per repetition; server restarted "
                  "before every run.",
        "run_id": run_id,
        "timestamp_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "machine_specs": machine_specs,
        "runtime_config": {
            "strategies": [s.label for s in STRATEGIES],
            "concurrency_levels": concurrency_levels,
            "workloads": workloads,
            "repetitions": args.runs,
            "requests_per_client": args.requests_per_client,
            "key_space": args.key_space,
            "value_size": args.value_size,
            "seed": args.seed,
        },
    }
    (output_dir / "metadata_rmit.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")

    print("\nRMIT run complete.")
    print(f"Raw data: {raw_rows_path}")
    print(f"Metadata: {output_dir / 'metadata_rmit.json'}")
    print("Next: python3 benchmarks/analyze_rmit_results.py --input", str(raw_rows_path))


if __name__ == "__main__":
    main()
