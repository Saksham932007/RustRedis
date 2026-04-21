#!/usr/bin/env python3
"""Run the full v12 experiment pipeline and generate publication artifacts.

This script executes:
- Full strategy x concurrency matrix (including new HdrHistogram and Sharded-N)
- Warmup-aware benchmark runs (handled by rustredis-bench)
- Sharded-2key anomaly investigation at 500 clients with CPU/throughput traces
- Raw + aggregated CSV export
- Four required publication-ready graphs
- Metadata + deposition README under experiment_results_v12/
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

try:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "matplotlib is required for graph generation. Install it with: pip install matplotlib"
    ) from exc


@dataclass(frozen=True)
class StrategySpec:
    key: str
    env_value: str
    label: str
    primary: bool = True


PRIMARY_STRATEGIES: List[StrategySpec] = [
    StrategySpec("disabled", "disabled", "Disabled"),
    StrategySpec("global_mutex", "global_mutex", "GlobalMutex"),
    StrategySpec("sharded_2key", "sharded_2key", "Sharded-2key"),
    StrategySpec("thread_local", "thread_local", "ThreadLocal"),
    StrategySpec("hdr_histogram", "hdr_histogram", "HdrHistogram"),
]

# Required additional configuration.
ADDITIONAL_STRATEGIES: List[StrategySpec] = [
    StrategySpec("sharded_n", "sharded_n", "Sharded-N", primary=False),
]

ALL_STRATEGIES: List[StrategySpec] = PRIMARY_STRATEGIES + ADDITIONAL_STRATEGIES
STRATEGY_ORDER: List[str] = [s.key for s in ALL_STRATEGIES]
STRATEGY_LABELS: Dict[str, str] = {s.key: s.label for s in ALL_STRATEGIES}

CONCURRENCY_LEVELS: List[int] = [100, 200, 300, 400, 500, 600, 700, 1000]

STRATEGY_COLORS: Dict[str, str] = {
    "disabled": "#1f4e79",
    "global_mutex": "#e07a5f",
    "sharded_2key": "#2a9d8f",
    "thread_local": "#6d597a",
    "hdr_histogram": "#f4a261",
    "sharded_n": "#3a7d44",
}

MARKERS: List[str] = ["o", "s", "^", "D", "v", "P"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run reproducible v12 experiment pipeline")
    parser.add_argument("--output-dir", default="experiment_results_v12", help="Deposition output directory")
    parser.add_argument("--runs", type=int, default=30, help="Runs per configuration")
    parser.add_argument(
        "--requests-per-client",
        type=int,
        default=1000,
        help="Requests per client per run (10%% warmup discarded by benchmark)",
    )
    parser.add_argument("--key-space", type=int, default=10000)
    parser.add_argument("--value-size", type=int, default=64)
    parser.add_argument("--cooldown-secs", type=float, default=3.0)
    parser.add_argument("--config-cooldown-secs", type=float, default=3.0)
    parser.add_argument("--run-retry-limit", type=int, default=5)
    parser.add_argument("--port", type=int, default=6379)
    parser.add_argument("--worker-threads", type=int, default=8)
    parser.add_argument("--server-startup-timeout-secs", type=int, default=180)
    parser.add_argument("--trace-interval-secs", type=float, default=0.2)
    parser.add_argument("--skip-build", action="store_true", help="Skip release build")
    parser.add_argument(
        "--run-id",
        default=None,
        help="Resume from an existing run_data/<run_id> directory",
    )
    parser.add_argument(
        "--force-rerun",
        action="store_true",
        help="Ignore existing per-configuration data and rerun everything",
    )
    return parser.parse_args()


def run_text_command(
    command: Sequence[str],
    cwd: Path,
    env: Optional[Dict[str, str]] = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(command),
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        check=check,
    )


def run_logged_command(
    command: Sequence[str],
    cwd: Path,
    log_path: Path,
    env: Optional[Dict[str, str]] = None,
) -> int:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8") as log_file:
        process = subprocess.run(
            list(command),
            cwd=cwd,
            env=env,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
    return process.returncode


def redis_ping(root_dir: Path, port: int) -> bool:
    result = run_text_command(
        ["redis-cli", "-p", str(port), "PING"],
        cwd=root_dir,
        check=False,
    )
    return result.returncode == 0 and "PONG" in (result.stdout or "")


def wait_for_server(root_dir: Path, port: int, server_proc: subprocess.Popen[str], timeout_secs: int) -> None:
    start = time.time()
    while True:
        if redis_ping(root_dir, port):
            return

        if server_proc.poll() is not None:
            raise RuntimeError(f"Server exited early with code {server_proc.returncode}")

        if time.time() - start >= timeout_secs:
            raise TimeoutError(f"Server did not become ready on port {port} within {timeout_secs}s")

        time.sleep(1.0)


def stop_server(server_proc: subprocess.Popen[str], log_handle) -> None:
    try:
        if server_proc.poll() is None:
            server_proc.terminate()
            try:
                server_proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server_proc.kill()
                server_proc.wait(timeout=10)
    finally:
        log_handle.close()


def parse_benchmark_run(run_json_path: Path) -> Dict[str, float]:
    payload = json.loads(run_json_path.read_text(encoding="utf-8"))
    if not payload.get("results"):
        raise ValueError(f"No results in {run_json_path}")

    result = payload["results"][0]
    per_run = result.get("per_run", [])
    row = per_run[0] if per_run else result

    parsed = {
        "throughput": float(row.get("ops_per_sec", 0.0)),
        "p99": float(row.get("p99_us", 0.0)),
        "median_latency": float(row.get("p50_us", 0.0)),
        "avg_latency": float(row.get("avg_us", 0.0)),
        "latency_stddev": float(row.get("latency_stddev_us", 0.0)),
        "latency_cv": float(row.get("latency_cv", 0.0)),
        "errors": int(row.get("errors", 0)),
        "warmup_ops_per_client": int(row.get("warmup_ops_per_client", 0)),
        "measured_ops_per_client": int(row.get("measured_ops_per_client", 0)),
    }
    return parsed


def write_csv(path: Path, rows: Iterable[Dict[str, object]], fieldnames: Sequence[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as fp:
        writer = csv.DictWriter(fp, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def mean(values: Sequence[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def stddev(values: Sequence[float]) -> float:
    if len(values) < 2:
        return 0.0
    return statistics.stdev(values)


def compute_stats(values: Sequence[float]) -> Dict[str, float]:
    n = len(values)
    if n == 0:
        return {
            "n": 0,
            "mean": 0.0,
            "stddev": 0.0,
            "median": 0.0,
            "cv": 0.0,
            "ci_low": 0.0,
            "ci_high": 0.0,
        }

    m = mean(values)
    sd = stddev(values)
    med = statistics.median(values)
    cv = (sd / m) if m > 0 else 0.0
    ci_margin = 1.96 * sd / math.sqrt(n) if n > 0 else 0.0
    return {
        "n": float(n),
        "mean": m,
        "stddev": sd,
        "median": med,
        "cv": cv,
        "ci_low": m - ci_margin,
        "ci_high": m + ci_margin,
    }


def get_total_commands_processed(root_dir: Path, port: int) -> Optional[int]:
    response = run_text_command(
        ["redis-cli", "-p", str(port), "STATS"],
        cwd=root_dir,
        check=False,
    )
    if response.returncode != 0:
        return None

    match = re.search(r"total_commands_processed:(\d+)", response.stdout)
    if not match:
        return None
    return int(match.group(1))


def get_cpu_percent(pid: int, root_dir: Path) -> Optional[float]:
    response = run_text_command(
        ["ps", "-p", str(pid), "-o", "%cpu="],
        cwd=root_dir,
        check=False,
    )
    if response.returncode != 0:
        return None

    text = (response.stdout or "").strip()
    if not text:
        return None

    try:
        return float(text)
    except ValueError:
        return None


def build_binaries(root_dir: Path) -> None:
    print("Building release binaries...")
    run_text_command(["cargo", "build", "--release", "--bin", "server"], cwd=root_dir)
    run_text_command(
        ["cargo", "build", "--release", "--manifest-path", "benchmarks/Cargo.toml"],
        cwd=root_dir,
    )


def start_server_for_strategy(
    root_dir: Path,
    strategy: StrategySpec,
    worker_threads: int,
    port: int,
    startup_timeout_secs: int,
    server_log_path: Path,
    extra_env: Optional[Dict[str, str]] = None,
) -> Tuple[subprocess.Popen[str], object]:
    env = os.environ.copy()
    env["TOKIO_WORKER_THREADS"] = str(worker_threads)
    env["RUSTREDIS_METRICS_STRATEGY"] = strategy.env_value
    env["RUSTREDIS_DISABLE_AOF"] = "1"
    if extra_env:
        env.update(extra_env)

    server_log_path.parent.mkdir(parents=True, exist_ok=True)
    log_handle = server_log_path.open("w", encoding="utf-8")
    server_proc = subprocess.Popen(
        ["./target/release/server"],
        cwd=root_dir,
        env=env,
        stdout=log_handle,
        stderr=subprocess.STDOUT,
        text=True,
    )

    try:
        wait_for_server(root_dir, port, server_proc, startup_timeout_secs)
    except Exception:
        stop_server(server_proc, log_handle)
        raise

    return server_proc, log_handle


def run_single_benchmark_invocation(
    root_dir: Path,
    port: int,
    concurrency: int,
    requests_per_client: int,
    key_space: int,
    value_size: int,
    run_dir: Path,
) -> int:
    command = [
        "./target/release/rustredis-bench",
        "--host",
        "127.0.0.1",
        "--port",
        str(port),
        "--concurrency",
        str(concurrency),
        "--requests",
        str(requests_per_client),
        "--runs",
        "1",
        "--workload",
        "mixed",
        "--key-space",
        str(key_space),
        "--value-size",
        str(value_size),
        "--output-dir",
        str(run_dir),
    ]
    return run_logged_command(command, cwd=root_dir, log_path=run_dir / "bench_stdout.log")


def run_configuration(
    root_dir: Path,
    output_run_root: Path,
    strategy: StrategySpec,
    concurrency: int,
    runs: int,
    requests_per_client: int,
    key_space: int,
    value_size: int,
    cooldown_secs: float,
    config_cooldown_secs: float,
    run_retry_limit: int,
    worker_threads: int,
    port: int,
    startup_timeout_secs: int,
) -> List[Dict[str, object]]:
    cfg_dir = output_run_root / strategy.key / f"c{concurrency}"
    cfg_dir.mkdir(parents=True, exist_ok=True)

    server_proc, server_log_handle = start_server_for_strategy(
        root_dir=root_dir,
        strategy=strategy,
        worker_threads=worker_threads,
        port=port,
        startup_timeout_secs=startup_timeout_secs,
        server_log_path=cfg_dir / "server.log",
    )

    rows: List[Dict[str, object]] = []
    try:
        for run_id in range(1, runs + 1):
            run_dir = cfg_dir / f"run_{run_id}"
            run_dir.mkdir(parents=True, exist_ok=True)

            run_success = False
            last_error: Optional[str] = None

            for attempt in range(1, run_retry_limit + 1):
                if run_dir.exists():
                    shutil.rmtree(run_dir)
                run_dir.mkdir(parents=True, exist_ok=True)

                rc = run_single_benchmark_invocation(
                    root_dir=root_dir,
                    port=port,
                    concurrency=concurrency,
                    requests_per_client=requests_per_client,
                    key_space=key_space,
                    value_size=value_size,
                    run_dir=run_dir,
                )

                run_json_path = run_dir / "benchmark_results.json"
                if rc == 0 and run_json_path.exists():
                    parsed = parse_benchmark_run(run_json_path)
                    row = {
                        "strategy": strategy.key,
                        "strategy_label": strategy.label,
                        "concurrency": concurrency,
                        "run_id": run_id,
                        "throughput": parsed["throughput"],
                        "p99": parsed["p99"],
                        "median_latency": parsed["median_latency"],
                        "avg_latency": parsed["avg_latency"],
                        "latency_stddev": parsed["latency_stddev"],
                        "latency_cv": parsed["latency_cv"],
                        "errors": parsed["errors"],
                        "warmup_ops_per_client": parsed["warmup_ops_per_client"],
                        "measured_ops_per_client": parsed["measured_ops_per_client"],
                    }
                    rows.append(row)
                    run_success = True
                    break

                last_error = (
                    f"run={run_id} attempt={attempt} failed (rc={rc}) for {strategy.key}/c{concurrency}"
                )
                time.sleep(cooldown_secs)

            if not run_success:
                raise RuntimeError(last_error or "benchmark run failed")

            time.sleep(cooldown_secs)

        cmdstat = run_text_command(
            ["redis-cli", "-p", str(port), "CMDSTAT"],
            cwd=root_dir,
            check=False,
        )
        (cfg_dir / "cmdstat.txt").write_text(cmdstat.stdout or "", encoding="utf-8")
    finally:
        stop_server(server_proc, server_log_handle)

    time.sleep(config_cooldown_secs)
    return rows


def validate_configuration_rows(
    rows: List[Dict[str, object]],
    expected_runs: int,
    expected_warmup: int,
    expected_measured: int,
) -> List[str]:
    issues: List[str] = []

    if len(rows) != expected_runs:
        issues.append(f"expected {expected_runs} runs, found {len(rows)}")

    for row in rows:
        rid = int(row["run_id"])
        p99 = float(row["p99"])
        med = float(row["median_latency"])
        std = float(row["latency_stddev"])
        warmup = int(row["warmup_ops_per_client"])
        measured = int(row["measured_ops_per_client"])

        if p99 < 0 or med < 0 or std < 0:
            issues.append(f"run {rid} has negative latency metric")
        if warmup != expected_warmup:
            issues.append(f"run {rid} warmup mismatch: {warmup} != {expected_warmup}")
        if measured != expected_measured:
            issues.append(f"run {rid} measured mismatch: {measured} != {expected_measured}")

    return issues


def run_configuration_with_retry(
    root_dir: Path,
    output_run_root: Path,
    strategy: StrategySpec,
    concurrency: int,
    runs: int,
    requests_per_client: int,
    key_space: int,
    value_size: int,
    cooldown_secs: float,
    config_cooldown_secs: float,
    run_retry_limit: int,
    worker_threads: int,
    port: int,
    startup_timeout_secs: int,
) -> List[Dict[str, object]]:
    expected_warmup = requests_per_client // 10
    expected_measured = requests_per_client - expected_warmup

    cfg_dir = output_run_root / strategy.key / f"c{concurrency}"
    for attempt in range(1, 3):
        if cfg_dir.exists():
            shutil.rmtree(cfg_dir)

        rows = run_configuration(
            root_dir=root_dir,
            output_run_root=output_run_root,
            strategy=strategy,
            concurrency=concurrency,
            runs=runs,
            requests_per_client=requests_per_client,
            key_space=key_space,
            value_size=value_size,
            cooldown_secs=cooldown_secs,
            config_cooldown_secs=config_cooldown_secs,
            run_retry_limit=run_retry_limit,
            worker_threads=worker_threads,
            port=port,
            startup_timeout_secs=startup_timeout_secs,
        )

        issues = validate_configuration_rows(rows, runs, expected_warmup, expected_measured)
        if not issues:
            return rows

        print(
            f"Validation failed for {strategy.key}/c{concurrency} attempt {attempt}: "
            + "; ".join(issues)
        )

    raise RuntimeError(f"Configuration validation failed after rerun: {strategy.key}/c{concurrency}")


def load_existing_configuration_rows(
    cfg_dir: Path,
    strategy: StrategySpec,
    concurrency: int,
    expected_runs: int,
) -> List[Dict[str, object]]:
    rows: List[Dict[str, object]] = []

    for run_id in range(1, expected_runs + 1):
        run_json_path = cfg_dir / f"run_{run_id}" / "benchmark_results.json"
        if not run_json_path.exists():
            return rows

        try:
            parsed = parse_benchmark_run(run_json_path)
        except Exception:
            return []

        rows.append(
            {
                "strategy": strategy.key,
                "strategy_label": strategy.label,
                "concurrency": concurrency,
                "run_id": run_id,
                "throughput": parsed["throughput"],
                "p99": parsed["p99"],
                "median_latency": parsed["median_latency"],
                "avg_latency": parsed["avg_latency"],
                "latency_stddev": parsed["latency_stddev"],
                "latency_cv": parsed["latency_cv"],
                "errors": parsed["errors"],
                "warmup_ops_per_client": parsed["warmup_ops_per_client"],
                "measured_ops_per_client": parsed["measured_ops_per_client"],
            }
        )

    return rows


def validate_full_matrix(
    rows: List[Dict[str, object]],
    strategies: Sequence[StrategySpec],
    conc_levels: Sequence[int],
    runs_per_config: int,
    expected_warmup: int,
    expected_measured: int,
) -> List[str]:
    issues: List[str] = []

    counts: Dict[Tuple[str, int], int] = {}
    for row in rows:
        key = (str(row["strategy"]), int(row["concurrency"]))
        counts[key] = counts.get(key, 0) + 1

        if float(row["p99"]) < 0 or float(row["median_latency"]) < 0:
            issues.append(
                f"negative latency in {row['strategy']}/c{row['concurrency']}/run_{row['run_id']}"
            )

        if int(row["warmup_ops_per_client"]) != expected_warmup:
            issues.append(
                f"warmup not discarded correctly in {row['strategy']}/c{row['concurrency']}/run_{row['run_id']}"
            )

        if int(row["measured_ops_per_client"]) != expected_measured:
            issues.append(
                f"measured count mismatch in {row['strategy']}/c{row['concurrency']}/run_{row['run_id']}"
            )

    for strategy in strategies:
        for concurrency in conc_levels:
            key = (strategy.key, concurrency)
            if counts.get(key, 0) != runs_per_config:
                issues.append(
                    f"run-count mismatch for {strategy.key}/c{concurrency}: {counts.get(key, 0)} != {runs_per_config}"
                )

    return issues


def investigate_sharded_2key_anomaly(
    root_dir: Path,
    anomaly_dir: Path,
    runs: int,
    requests_per_client: int,
    key_space: int,
    value_size: int,
    cooldown_secs: float,
    run_retry_limit: int,
    worker_threads: int,
    port: int,
    startup_timeout_secs: int,
    trace_interval_secs: float,
) -> List[Dict[str, object]]:
    strategy = StrategySpec("sharded_2key", "sharded_2key", "Sharded-2key")
    concurrency = 500

    server_proc, server_log_handle = start_server_for_strategy(
        root_dir=root_dir,
        strategy=strategy,
        worker_threads=worker_threads,
        port=port,
        startup_timeout_secs=startup_timeout_secs,
        server_log_path=anomaly_dir / "server_debug.log",
        extra_env={"RUST_LOG": "debug"},
    )

    summary_rows: List[Dict[str, object]] = []
    try:
        for run_id in range(1, runs + 1):
            run_dir = anomaly_dir / f"run_{run_id}"
            if run_dir.exists():
                shutil.rmtree(run_dir)
            run_dir.mkdir(parents=True, exist_ok=True)

            attempt_success = False
            last_error: Optional[str] = None

            for attempt in range(1, run_retry_limit + 1):
                if run_dir.exists():
                    shutil.rmtree(run_dir)
                run_dir.mkdir(parents=True, exist_ok=True)

                bench_log = run_dir / "bench_stdout.log"
                command = [
                    "./target/release/rustredis-bench",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    str(port),
                    "--concurrency",
                    str(concurrency),
                    "--requests",
                    str(requests_per_client),
                    "--runs",
                    "1",
                    "--workload",
                    "mixed",
                    "--key-space",
                    str(key_space),
                    "--value-size",
                    str(value_size),
                    "--output-dir",
                    str(run_dir),
                ]

                with bench_log.open("w", encoding="utf-8") as log_file:
                    bench_proc = subprocess.Popen(
                        command,
                        cwd=root_dir,
                        stdout=log_file,
                        stderr=subprocess.STDOUT,
                        text=True,
                    )

                    trace_rows: List[Dict[str, object]] = []
                    prev_total: Optional[int] = None
                    prev_ts: Optional[float] = None
                    trace_start = time.time()

                    while bench_proc.poll() is None:
                        now = time.time()
                        elapsed = now - trace_start
                        cpu = get_cpu_percent(server_proc.pid, root_dir)
                        total_cmd = get_total_commands_processed(root_dir, port)

                        throughput_inst: Optional[float] = None
                        if (
                            prev_total is not None
                            and total_cmd is not None
                            and prev_ts is not None
                            and now > prev_ts
                        ):
                            throughput_inst = (total_cmd - prev_total) / (now - prev_ts)

                        trace_rows.append(
                            {
                                "elapsed_secs": f"{elapsed:.6f}",
                                "cpu_percent": "" if cpu is None else f"{cpu:.4f}",
                                "total_commands_processed": "" if total_cmd is None else str(total_cmd),
                                "throughput_ops_sec": ""
                                if throughput_inst is None
                                else f"{throughput_inst:.6f}",
                            }
                        )

                        prev_total = total_cmd if total_cmd is not None else prev_total
                        prev_ts = now
                        time.sleep(trace_interval_secs)

                    rc = bench_proc.wait()

                run_json_path = run_dir / "benchmark_results.json"
                if rc == 0 and run_json_path.exists():
                    write_csv(
                        run_dir / "throughput_trace.csv",
                        trace_rows,
                        [
                            "elapsed_secs",
                            "cpu_percent",
                            "total_commands_processed",
                            "throughput_ops_sec",
                        ],
                    )

                    parsed = parse_benchmark_run(run_json_path)
                    cpu_values = [
                        float(row["cpu_percent"])
                        for row in trace_rows
                        if row.get("cpu_percent") not in (None, "")
                    ]
                    throughput_values = [
                        float(row["throughput_ops_sec"])
                        for row in trace_rows
                        if row.get("throughput_ops_sec") not in (None, "")
                    ]

                    summary_rows.append(
                        {
                            "run_id": run_id,
                            "throughput": parsed["throughput"],
                            "p99": parsed["p99"],
                            "cpu_mean": mean(cpu_values),
                            "cpu_max": max(cpu_values) if cpu_values else 0.0,
                            "trace_throughput_mean": mean(throughput_values),
                            "trace_throughput_max": max(throughput_values) if throughput_values else 0.0,
                            "trace_samples": len(trace_rows),
                        }
                    )
                    attempt_success = True
                    break

                last_error = f"anomaly run {run_id} attempt {attempt} failed (rc={rc})"
                time.sleep(cooldown_secs)

            if not attempt_success:
                raise RuntimeError(last_error or "anomaly run failed")

            time.sleep(cooldown_secs)
    finally:
        stop_server(server_proc, server_log_handle)

    write_csv(
        anomaly_dir / "anomaly_summary.csv",
        summary_rows,
        [
            "run_id",
            "throughput",
            "p99",
            "cpu_mean",
            "cpu_max",
            "trace_throughput_mean",
            "trace_throughput_max",
            "trace_samples",
        ],
    )

    return summary_rows


def decide_outlier_policy(
    main_rows: List[Dict[str, object]],
    anomaly_rows: List[Dict[str, object]],
) -> Dict[str, object]:
    target_rows = sorted(
        [
            row
            for row in main_rows
            if row["strategy"] == "sharded_2key" and int(row["concurrency"]) == 500
        ],
        key=lambda row: int(row["run_id"]),
    )

    if not target_rows:
        return {
            "decision": "insufficient_data",
            "details": "No sharded_2key/c500 rows available for anomaly policy.",
            "excluded_runs": [],
        }

    main_values = [float(row["throughput"]) for row in target_rows]
    main_run1 = float(target_rows[0]["throughput"])
    main_median = statistics.median(main_values)

    rerun_values = [float(row["throughput"]) for row in anomaly_rows]
    rerun_median = statistics.median(rerun_values) if rerun_values else 0.0
    rerun_run1 = rerun_values[0] if rerun_values else 0.0

    rerun_threshold = 3.0 * rerun_median if rerun_median > 0 else math.inf
    main_threshold = 3.0 * main_median if main_median > 0 else math.inf

    anomaly_repeats = abs(rerun_run1) > rerun_threshold
    original_is_outlier = abs(main_run1) > main_threshold

    if anomaly_repeats:
        decision = "unstable_system_behavior"
        details = "Rerun anomaly repeated at run 1 (> 3x median); marked as unstable system behavior."
        excluded_runs: List[int] = []
    elif original_is_outlier:
        decision = "original_outlier_excluded"
        details = "Original sharded_2key/c500 run 1 classified as outlier (|value| > 3x median) and excluded from aggregated analysis."
        excluded_runs = [1]
    else:
        decision = "no_formal_outlier"
        details = "Rerun anomaly did not repeat and original run does not violate |value| > 3x median rule."
        excluded_runs = []

    return {
        "decision": decision,
        "details": details,
        "excluded_runs": excluded_runs,
        "main_run1": main_run1,
        "main_median": main_median,
        "main_threshold_3x_median": main_threshold,
        "rerun_run1": rerun_run1,
        "rerun_median": rerun_median,
        "rerun_threshold_3x_median": rerun_threshold,
    }


def aggregate_results(
    rows: List[Dict[str, object]],
    strategies: Sequence[StrategySpec],
    conc_levels: Sequence[int],
    exclusions: Dict[Tuple[str, int], List[int]],
) -> List[Dict[str, object]]:
    aggregated: List[Dict[str, object]] = []

    for strategy in strategies:
        for concurrency in conc_levels:
            cfg_rows = sorted(
                [
                    row
                    for row in rows
                    if row["strategy"] == strategy.key and int(row["concurrency"]) == concurrency
                ],
                key=lambda row: int(row["run_id"]),
            )

            excluded = set(exclusions.get((strategy.key, concurrency), []))
            analysis_rows = [row for row in cfg_rows if int(row["run_id"]) not in excluded]

            throughput_values = [float(row["throughput"]) for row in analysis_rows]
            p99_values = [float(row["p99"]) for row in analysis_rows]

            throughput_stats = compute_stats(throughput_values)
            p99_stats = compute_stats(p99_values)

            aggregated.append(
                {
                    "strategy": strategy.key,
                    "strategy_label": strategy.label,
                    "concurrency": concurrency,
                    "n": int(throughput_stats["n"]),
                    "mean": throughput_stats["mean"],
                    "stddev": throughput_stats["stddev"],
                    "median": throughput_stats["median"],
                    "CV": throughput_stats["cv"],
                    "CI_low": throughput_stats["ci_low"],
                    "CI_high": throughput_stats["ci_high"],
                    "p99_mean": p99_stats["mean"],
                    "p99_stddev": p99_stats["stddev"],
                    "p99_median": p99_stats["median"],
                    "p99_CV": p99_stats["cv"],
                    "p99_CI_low": p99_stats["ci_low"],
                    "p99_CI_high": p99_stats["ci_high"],
                    "excluded_runs": "|".join(str(x) for x in sorted(excluded)),
                }
            )

    return aggregated


def validate_aggregated_ci(aggregated_rows: List[Dict[str, object]]) -> List[str]:
    issues: List[str] = []
    for row in aggregated_rows:
        n = int(row["n"])
        mean_v = float(row["mean"])
        std_v = float(row["stddev"])
        ci_low = float(row["CI_low"])
        ci_high = float(row["CI_high"])

        if n <= 0:
            continue

        margin_expected = 1.96 * std_v / math.sqrt(n)
        margin_low = mean_v - ci_low
        margin_high = ci_high - mean_v
        if not (math.isclose(margin_low, margin_expected, rel_tol=1e-9, abs_tol=1e-9) and math.isclose(margin_high, margin_expected, rel_tol=1e-9, abs_tol=1e-9)):
            issues.append(
                f"CI mismatch for {row['strategy']}/c{row['concurrency']}: expected margin {margin_expected}, got low={margin_low}, high={margin_high}"
            )

    return issues


def configure_plot_style() -> None:
    plt.style.use("default")
    plt.rcParams.update(
        {
            "figure.facecolor": "#ffffff",
            "axes.facecolor": "#fbfbfb",
            "savefig.facecolor": "#ffffff",
            "font.family": "sans-serif",
            "font.sans-serif": ["Avenir", "Helvetica", "DejaVu Sans"],
            "font.size": 12,
            "axes.labelsize": 13,
            "axes.titlesize": 15,
            "legend.fontsize": 12,
            "xtick.labelsize": 12,
            "ytick.labelsize": 12,
            "axes.spines.top": False,
            "axes.spines.right": False,
        }
    )


def grouped_rows(aggregated_rows: List[Dict[str, object]]) -> Dict[str, List[Dict[str, object]]]:
    grouped: Dict[str, List[Dict[str, object]]] = {key: [] for key in STRATEGY_ORDER}
    for row in aggregated_rows:
        grouped[str(row["strategy"])].append(row)

    for strategy in grouped:
        grouped[strategy].sort(key=lambda row: int(row["concurrency"]))
    return grouped


def make_throughput_plot(aggregated_rows: List[Dict[str, object]], out_path: Path) -> None:
    grouped = grouped_rows(aggregated_rows)
    fig, ax = plt.subplots(figsize=(10, 6))

    for idx, strategy in enumerate(STRATEGY_ORDER):
        rows = grouped.get(strategy, [])
        if not rows:
            continue
        x = [int(row["concurrency"]) for row in rows]
        y = [float(row["mean"]) for row in rows]
        yerr = [max(0.0, float(row["CI_high"]) - float(row["mean"])) for row in rows]

        ax.errorbar(
            x,
            y,
            yerr=yerr,
            color=STRATEGY_COLORS[strategy],
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=2.2,
            markersize=6,
            capsize=4,
            label=STRATEGY_LABELS[strategy],
        )

    ax.set_title("Throughput vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("Throughput (ops/sec)")
    ax.set_xticks(CONCURRENCY_LEVELS)
    ax.grid(axis="y", alpha=0.2, linewidth=0.8)
    ax.legend(frameon=False, ncols=2)

    fig.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)


def make_latency_plot(aggregated_rows: List[Dict[str, object]], out_path: Path) -> None:
    grouped = grouped_rows(aggregated_rows)
    fig, ax = plt.subplots(figsize=(10, 6))

    for idx, strategy in enumerate(STRATEGY_ORDER):
        rows = grouped.get(strategy, [])
        if not rows:
            continue
        x = [int(row["concurrency"]) for row in rows]
        y = [float(row["p99_mean"]) for row in rows]

        ax.plot(
            x,
            y,
            color=STRATEGY_COLORS[strategy],
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=2.2,
            markersize=6,
            label=STRATEGY_LABELS[strategy],
        )

    ax.set_title("p99 Latency vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("p99 latency (us)")
    ax.set_xticks(CONCURRENCY_LEVELS)
    ax.set_ylim(bottom=0)
    ax.grid(axis="y", alpha=0.2, linewidth=0.8)
    ax.legend(frameon=False, ncols=2)

    fig.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)


def make_cv_plot(aggregated_rows: List[Dict[str, object]], out_path: Path) -> None:
    grouped = grouped_rows(aggregated_rows)
    fig, ax = plt.subplots(figsize=(10, 6))

    for idx, strategy in enumerate(STRATEGY_ORDER):
        rows = grouped.get(strategy, [])
        if not rows:
            continue
        x = [int(row["concurrency"]) for row in rows]
        y = [float(row["CV"]) for row in rows]

        ax.plot(
            x,
            y,
            color=STRATEGY_COLORS[strategy],
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=2.2,
            markersize=6,
            label=STRATEGY_LABELS[strategy],
        )

    ax.set_title("Coefficient of Variation vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("CV")
    ax.set_xticks(CONCURRENCY_LEVELS)
    ax.grid(axis="y", alpha=0.2, linewidth=0.8)
    ax.legend(frameon=False, ncols=2)

    fig.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)


def make_boxplot(
    raw_rows: List[Dict[str, object]],
    exclusions: Dict[Tuple[str, int], List[int]],
    out_path: Path,
) -> None:
    fig, ax = plt.subplots(figsize=(10, 6))

    box_data: List[List[float]] = []
    labels: List[str] = []
    colors: List[str] = []

    for strategy in STRATEGY_ORDER:
        excluded = set(exclusions.get((strategy, 500), []))
        values = [
            float(row["throughput"])
            for row in raw_rows
            if row["strategy"] == strategy
            and int(row["concurrency"]) == 500
            and int(row["run_id"]) not in excluded
        ]
        if values:
            box_data.append(values)
            labels.append(STRATEGY_LABELS[strategy])
            colors.append(STRATEGY_COLORS[strategy])

    if not box_data:
        raise RuntimeError("No data available for throughput distribution boxplot at 500 clients")

    box = ax.boxplot(box_data, labels=labels, patch_artist=True, showfliers=True, widths=0.55)
    for patch, color in zip(box["boxes"], colors):
        patch.set_facecolor(color)
        patch.set_alpha(0.35)
        patch.set_edgecolor(color)

    ax.set_title("Throughput Distribution at 500 Clients")
    ax.set_xlabel("Strategy")
    ax.set_ylabel("Throughput (ops/sec)")
    ax.grid(axis="y", alpha=0.2, linewidth=0.8)

    fig.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)


def collect_machine_specs(root_dir: Path) -> Dict[str, object]:
    def cmd_value(command: Sequence[str]) -> str:
        result = run_text_command(command, cwd=root_dir, check=False)
        return (result.stdout or "").strip()

    return {
        "hostname": platform.node(),
        "platform": platform.platform(),
        "processor": platform.processor(),
        "python_version": platform.python_version(),
        "cpu_brand": cmd_value(["sysctl", "-n", "machdep.cpu.brand_string"]),
        "logicalcpu": cmd_value(["sysctl", "-n", "hw.logicalcpu"]),
        "physicalcpu": cmd_value(["sysctl", "-n", "hw.physicalcpu"]),
        "mem_bytes": cmd_value(["sysctl", "-n", "hw.memsize"]),
        "rustc": cmd_value(["rustc", "--version"]),
        "cargo": cmd_value(["cargo", "--version"]),
        "commit_hash": cmd_value(["git", "rev-parse", "HEAD"]),
    }


def write_metadata(
    metadata_path: Path,
    args: argparse.Namespace,
    machine_specs: Dict[str, object],
    run_timestamp: str,
    anomaly_decision: Dict[str, object],
) -> None:
    metadata = {
        "timestamp_utc": run_timestamp,
        "machine_specs": machine_specs,
        "runtime_config": {
            "language": "Rust",
            "runtime": "Tokio multi-thread",
            "worker_threads": args.worker_threads,
            "workload": "50% GET / 50% SET",
            "keyspace": args.key_space,
            "value_size": args.value_size,
            "requests_per_client": args.requests_per_client,
            "runs_per_configuration": args.runs,
            "cooldown_secs": args.cooldown_secs,
            "strategies_primary": [s.label for s in PRIMARY_STRATEGIES],
            "strategies_additional": [s.label for s in ADDITIONAL_STRATEGIES],
            "concurrency_levels": CONCURRENCY_LEVELS,
        },
        "anomaly_policy": anomaly_decision,
    }
    metadata_path.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def write_readme(
    readme_path: Path,
    run_data_subdir: Path,
    anomaly_decision: Dict[str, object],
) -> None:
    content = f"""# experiment_results_v12

This directory contains reproducible v12 experiment artifacts.

## Contents

- `raw_data.csv`: per-run data (`strategy, concurrency, run_id, throughput, p99`, plus additional diagnostics)
- `aggregated_data.csv`: per-configuration statistics (mean, stddev, median, CV, 95% CI)
- `metadata.json`: machine specs, runtime config, timestamp, commit hash
- `graphs/`: publication-quality PNG figures
- `run_data/`: detailed per-run logs and raw benchmark JSON
- `anomaly_investigation/`: sharded-2key/c500 rerun with debug logging, CPU traces, and throughput traces

## Graphs

- `graphs/throughput_vs_concurrency.png`
- `graphs/latency_vs_concurrency.png`
- `graphs/cv_vs_concurrency.png`
- `graphs/throughput_distribution.png`

## Outlier/Anomaly Decision

- Decision: {anomaly_decision.get('decision', 'unknown')}
- Details: {anomaly_decision.get('details', 'n/a')}
- Excluded runs for aggregated analysis: {anomaly_decision.get('excluded_runs', [])}

## Raw Logs

Detailed run logs are available under:

- `{run_data_subdir}`
"""
    readme_path.write_text(content, encoding="utf-8")


def main() -> None:
    args = parse_args()
    root_dir = Path(__file__).resolve().parents[1]

    output_dir = (root_dir / args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    run_timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    run_id = args.run_id or datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")

    run_data_root = output_dir / "run_data" / run_id
    anomaly_dir = output_dir / "anomaly_investigation"
    graphs_dir = output_dir / "graphs"

    if redis_ping(root_dir, args.port):
        raise SystemExit(
            f"Port {args.port} already responds to PING. Stop existing server before running v12 pipeline."
        )

    if not args.skip_build:
        build_binaries(root_dir)

    print("Running full v12 matrix...")
    print(f"Run ID: {run_id}")
    raw_rows: List[Dict[str, object]] = []

    for strategy in ALL_STRATEGIES:
        for concurrency in CONCURRENCY_LEVELS:
            cfg_dir = run_data_root / strategy.key / f"c{concurrency}"

            if not args.force_rerun and cfg_dir.exists():
                existing_rows = load_existing_configuration_rows(
                    cfg_dir=cfg_dir,
                    strategy=strategy,
                    concurrency=concurrency,
                    expected_runs=args.runs,
                )
                issues = validate_configuration_rows(
                    existing_rows,
                    expected_runs=args.runs,
                    expected_warmup=args.requests_per_client // 10,
                    expected_measured=args.requests_per_client - (args.requests_per_client // 10),
                )

                if len(existing_rows) == args.runs and not issues:
                    print(
                        f"  -> strategy={strategy.key:12s} concurrency={concurrency} (resume: using existing data)"
                    )
                    raw_rows.extend(existing_rows)
                    continue

                print(
                    f"  -> strategy={strategy.key:12s} concurrency={concurrency} (resume: existing data incomplete, rerunning)"
                )
            else:
                print(f"  -> strategy={strategy.key:12s} concurrency={concurrency}")

            cfg_rows = run_configuration_with_retry(
                root_dir=root_dir,
                output_run_root=run_data_root,
                strategy=strategy,
                concurrency=concurrency,
                runs=args.runs,
                requests_per_client=args.requests_per_client,
                key_space=args.key_space,
                value_size=args.value_size,
                cooldown_secs=args.cooldown_secs,
                config_cooldown_secs=args.config_cooldown_secs,
                run_retry_limit=args.run_retry_limit,
                worker_threads=args.worker_threads,
                port=args.port,
                startup_timeout_secs=args.server_startup_timeout_secs,
            )
            raw_rows.extend(cfg_rows)

    expected_warmup = args.requests_per_client // 10
    expected_measured = args.requests_per_client - expected_warmup
    matrix_issues = validate_full_matrix(
        raw_rows,
        ALL_STRATEGIES,
        CONCURRENCY_LEVELS,
        args.runs,
        expected_warmup,
        expected_measured,
    )
    if matrix_issues:
        issue_text = "\n".join(f"- {msg}" for msg in matrix_issues)
        raise RuntimeError(f"Validation failed after matrix run:\n{issue_text}")

    print("Running sharded-2key anomaly investigation at 500 clients...")
    if anomaly_dir.exists():
        shutil.rmtree(anomaly_dir)
    anomaly_dir.mkdir(parents=True, exist_ok=True)

    anomaly_rows = investigate_sharded_2key_anomaly(
        root_dir=root_dir,
        anomaly_dir=anomaly_dir,
        runs=args.runs,
        requests_per_client=args.requests_per_client,
        key_space=args.key_space,
        value_size=args.value_size,
        cooldown_secs=args.cooldown_secs,
        run_retry_limit=args.run_retry_limit,
        worker_threads=args.worker_threads,
        port=args.port,
        startup_timeout_secs=args.server_startup_timeout_secs,
        trace_interval_secs=args.trace_interval_secs,
    )

    anomaly_decision = decide_outlier_policy(raw_rows, anomaly_rows)
    exclusions: Dict[Tuple[str, int], List[int]] = {}
    excluded_runs = anomaly_decision.get("excluded_runs", [])
    if excluded_runs:
        exclusions[("sharded_2key", 500)] = list(excluded_runs)

    aggregated_rows = aggregate_results(raw_rows, ALL_STRATEGIES, CONCURRENCY_LEVELS, exclusions)

    ci_issues = validate_aggregated_ci(aggregated_rows)
    if ci_issues:
        issue_text = "\n".join(f"- {msg}" for msg in ci_issues)
        raise RuntimeError(f"CI validation failed:\n{issue_text}")

    raw_rows_sorted = sorted(
        raw_rows,
        key=lambda row: (
            STRATEGY_ORDER.index(str(row["strategy"])),
            int(row["concurrency"]),
            int(row["run_id"]),
        ),
    )

    write_csv(
        output_dir / "raw_data.csv",
        raw_rows_sorted,
        [
            "strategy",
            "strategy_label",
            "concurrency",
            "run_id",
            "throughput",
            "p99",
            "median_latency",
            "avg_latency",
            "latency_stddev",
            "latency_cv",
            "errors",
            "warmup_ops_per_client",
            "measured_ops_per_client",
        ],
    )

    aggregated_sorted = sorted(
        aggregated_rows,
        key=lambda row: (
            STRATEGY_ORDER.index(str(row["strategy"])),
            int(row["concurrency"]),
        ),
    )

    write_csv(
        output_dir / "aggregated_data.csv",
        aggregated_sorted,
        [
            "strategy",
            "strategy_label",
            "concurrency",
            "n",
            "mean",
            "stddev",
            "median",
            "CV",
            "CI_low",
            "CI_high",
            "p99_mean",
            "p99_stddev",
            "p99_median",
            "p99_CV",
            "p99_CI_low",
            "p99_CI_high",
            "excluded_runs",
        ],
    )

    configure_plot_style()
    graphs_dir.mkdir(parents=True, exist_ok=True)

    make_throughput_plot(aggregated_sorted, graphs_dir / "throughput_vs_concurrency.png")
    make_latency_plot(aggregated_sorted, graphs_dir / "latency_vs_concurrency.png")
    make_cv_plot(aggregated_sorted, graphs_dir / "cv_vs_concurrency.png")
    make_boxplot(raw_rows_sorted, exclusions, graphs_dir / "throughput_distribution.png")

    required_graphs = [
        graphs_dir / "throughput_vs_concurrency.png",
        graphs_dir / "latency_vs_concurrency.png",
        graphs_dir / "cv_vs_concurrency.png",
        graphs_dir / "throughput_distribution.png",
    ]
    missing_graphs = [str(path) for path in required_graphs if not path.exists()]
    if missing_graphs:
        raise RuntimeError("Graph generation failed; missing files: " + ", ".join(missing_graphs))

    machine_specs = collect_machine_specs(root_dir)
    write_metadata(output_dir / "metadata.json", args, machine_specs, run_timestamp, anomaly_decision)

    (output_dir / "anomaly_decision.json").write_text(
        json.dumps(anomaly_decision, indent=2) + "\n",
        encoding="utf-8",
    )

    write_readme(output_dir / "README.md", run_data_root, anomaly_decision)

    print("v12 pipeline complete.")
    print(f"Output directory: {output_dir}")


if __name__ == "__main__":
    main()
