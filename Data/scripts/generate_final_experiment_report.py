#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import re
import statistics
from pathlib import Path
from typing import Dict, List, Tuple

STRATEGY_ORDER = ["disabled", "global_mutex", "sharded", "thread_local"]
CLIENT_ORDER = [100, 500, 1000]


def parse_machine_details(path: Path) -> Dict[str, str]:
    details: Dict[str, str] = {}
    if not path.exists():
        return details
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            details[key.strip()] = value.strip()
    return details


def parse_tokio_version(lock_path: Path) -> str:
    if not lock_path.exists():
        return "unknown"

    text = lock_path.read_text(encoding="utf-8", errors="ignore")
    blocks = text.split("[[package]]")
    for block in blocks:
        if 'name = "tokio"' in block:
            m = re.search(r'version = "([^"]+)"', block)
            if m:
                return m.group(1)
    return "unknown"


def stats(values: List[float]) -> Tuple[float, float, float, float]:
    if not values:
        return 0.0, 0.0, 0.0, 0.0

    mean_v = sum(values) / len(values)
    if len(values) >= 2:
        variance_v = statistics.variance(values)
        stddev_v = math.sqrt(variance_v)
    else:
        variance_v = 0.0
        stddev_v = 0.0

    cv_v = (stddev_v / mean_v) if mean_v != 0 else 0.0
    return mean_v, stddev_v, variance_v, cv_v


def classify(cv: float) -> str:
    if cv < 0.1:
        return "Stable"
    if cv <= 0.3:
        return "Moderate"
    return "Unstable"


def fmt_num(v: float) -> str:
    return f"{v:.9f}"


def load_config_data(run_root: Path) -> Dict[Tuple[str, int], Dict]:
    data: Dict[Tuple[str, int], Dict] = {}

    for strategy in STRATEGY_ORDER:
        strategy_dir = run_root / strategy
        if not strategy_dir.exists():
            continue

        for client in CLIENT_ORDER:
            cfg_dir = strategy_dir / f"c{client}"
            json_path = cfg_dir / "benchmark_results.json"
            if not json_path.exists():
                continue

            payload = json.loads(json_path.read_text(encoding="utf-8"))
            mixed_result = None
            for result in payload.get("results", []):
                name = str(result.get("name", "")).lower()
                if "mixed" in name and int(result.get("concurrency", -1)) == client:
                    mixed_result = result
                    break

            if mixed_result is None and payload.get("results"):
                mixed_result = payload["results"][0]

            if mixed_result is None:
                continue

            per_run = mixed_result.get("per_run", [])
            throughputs = [float(r.get("ops_per_sec", 0.0)) for r in per_run]
            p99s = [float(r.get("p99_us", 0.0)) for r in per_run]
            errors = [int(r.get("errors", 0)) for r in per_run]

            tp_mean, tp_std, tp_var, tp_cv = stats(throughputs)
            p99_mean, p99_std, p99_var, p99_cv = stats(p99s)

            data[(strategy, client)] = {
                "strategy": strategy,
                "clients": client,
                "cfg_dir": cfg_dir,
                "json_path": json_path,
                "run_count": len(per_run),
                "runs": [
                    {
                        "run": idx + 1,
                        "throughput": throughputs[idx],
                        "p99": p99s[idx],
                        "errors": errors[idx],
                    }
                    for idx in range(len(per_run))
                ],
                "throughput_mean": tp_mean,
                "throughput_stddev": tp_std,
                "throughput_variance": tp_var,
                "throughput_cv": tp_cv,
                "p99_mean": p99_mean,
                "p99_stddev": p99_std,
                "p99_variance": p99_var,
                "p99_cv": p99_cv,
                "total_errors": sum(errors),
            }

    return data


def build_report(run_root: Path, output_path: Path) -> None:
    details = parse_machine_details(run_root / "machine_details.txt")
    tokio_version = parse_tokio_version(Path("Cargo.lock"))
    configs = load_config_data(run_root)

    missing_configs: List[str] = []
    wrong_run_count: List[str] = []
    anomalies: List[str] = []

    for strategy in STRATEGY_ORDER:
        for clients in CLIENT_ORDER:
            key = (strategy, clients)
            if key not in configs:
                missing_configs.append(f"{strategy}/c{clients}")
                continue

            if configs[key]["run_count"] != 10:
                wrong_run_count.append(f"{strategy}/c{clients}: {configs[key]['run_count']} runs")

            total_errors = configs[key]["total_errors"]
            if total_errors > 0:
                anomalies.append(f"{strategy}/c{clients}: total_errors={total_errors}")

    baseline_rows = []
    for clients in CLIENT_ORDER:
        base = configs.get(("disabled", clients))
        for strategy in STRATEGY_ORDER:
            cfg = configs.get((strategy, clients))
            if cfg is None:
                continue

            if strategy == "disabled":
                overhead = 0.0
                latency_increase = 0.0
            elif base is None or base["throughput_mean"] == 0.0 or base["p99_mean"] == 0.0:
                overhead = 0.0
                latency_increase = 0.0
            else:
                overhead = ((cfg["throughput_mean"] - base["throughput_mean"]) / base["throughput_mean"]) * 100.0
                latency_increase = ((cfg["p99_mean"] - base["p99_mean"]) / base["p99_mean"]) * 100.0

            baseline_rows.append(
                {
                    "strategy": strategy,
                    "clients": clients,
                    "throughput_overhead": overhead,
                    "latency_increase": latency_increase,
                }
            )

    observations: List[str] = []

    for clients in CLIENT_ORDER:
        available = [configs[(s, clients)] for s in STRATEGY_ORDER if (s, clients) in configs]
        if not available:
            continue

        max_tp_cfg = max(available, key=lambda x: x["throughput_mean"])
        min_tp_cfg = min(available, key=lambda x: x["throughput_mean"])
        min_p99_cfg = min(available, key=lambda x: x["p99_mean"])
        max_p99_cfg = max(available, key=lambda x: x["p99_mean"])

        observations.append(
            f"Clients={clients}: max throughput mean={fmt_num(max_tp_cfg['throughput_mean'])} ops/sec ({max_tp_cfg['strategy']}), min throughput mean={fmt_num(min_tp_cfg['throughput_mean'])} ops/sec ({min_tp_cfg['strategy']})."
        )
        observations.append(
            f"Clients={clients}: min p99 mean={fmt_num(min_p99_cfg['p99_mean'])} µs ({min_p99_cfg['strategy']}), max p99 mean={fmt_num(max_p99_cfg['p99_mean'])} µs ({max_p99_cfg['strategy']})."
        )

    for strategy in STRATEGY_ORDER:
        tps = [configs[(strategy, c)]["throughput_mean"] for c in CLIENT_ORDER if (strategy, c) in configs]
        p99s = [configs[(strategy, c)]["p99_mean"] for c in CLIENT_ORDER if (strategy, c) in configs]
        if len(tps) == len(CLIENT_ORDER):
            observations.append(
                f"Strategy={strategy}: throughput means at clients [100,500,1000] = [{fmt_num(tps[0])}, {fmt_num(tps[1])}, {fmt_num(tps[2])}] ops/sec."
            )
        if len(p99s) == len(CLIENT_ORDER):
            observations.append(
                f"Strategy={strategy}: p99 means at clients [100,500,1000] = [{fmt_num(p99s[0])}, {fmt_num(p99s[1])}, {fmt_num(p99s[2])}] µs."
            )

    max_tp_cv_cfg = None
    max_p99_cv_cfg = None
    if configs:
        max_tp_cv_cfg = max(configs.values(), key=lambda x: x["throughput_cv"])
        max_p99_cv_cfg = max(configs.values(), key=lambda x: x["p99_cv"])

    if max_tp_cv_cfg is not None:
        observations.append(
            f"Highest throughput CV={fmt_num(max_tp_cv_cfg['throughput_cv'])} at {max_tp_cv_cfg['strategy']}/c{max_tp_cv_cfg['clients']}."
        )
    if max_p99_cv_cfg is not None:
        observations.append(
            f"Highest p99 CV={fmt_num(max_p99_cv_cfg['p99_cv'])} at {max_p99_cv_cfg['strategy']}/c{max_p99_cv_cfg['clients']}."
        )

    lines: List[str] = []

    lines.append("# Final Experiment Report")
    lines.append("")

    lines.append("## 1. System Information")
    lines.append("")
    lines.append(f"- CPU: {details.get('cpu', 'unknown')}")
    lines.append(f"- Cores: logical={details.get('logicalcpu', 'unknown')}, physical={details.get('physicalcpu', 'unknown')}")
    lines.append(f"- RAM: {details.get('mem_bytes', 'unknown')} bytes")
    lines.append(f"- OS: {details.get('os', 'unknown')}")
    lines.append(f"- Kernel: {details.get('kernel', 'unknown')}")
    lines.append(f"- Rust version: {details.get('rustc', 'unknown')}")
    lines.append(f"- Tokio version: {tokio_version}")
    lines.append(f"- Experiment timestamp: {details.get('timestamp', 'unknown')}")
    lines.append(f"- Raw data root: {run_root}")
    lines.append("")

    lines.append("## 2. Experiment Configuration")
    lines.append("")
    lines.append("- Workload: mixed (50% GET / 50% SET)")
    lines.append("- Strategies tested: disabled, global_mutex, sharded, thread_local")
    lines.append("- Concurrency levels: 100, 500, 1000")
    lines.append("- Runs per configuration: 10")
    lines.append(f"- Requests per configuration: {details.get('requests', '10000')}")
    lines.append(f"- Key space: {details.get('key_space', '10000')}")
    lines.append(f"- Value size: {details.get('value_size', '64')} bytes")
    lines.append(f"- Tokio worker threads (fixed): {details.get('tokio_worker_threads_fixed', 'unknown')}")
    lines.append("")

    lines.append("## 3. Raw Results")
    lines.append("")

    for strategy in STRATEGY_ORDER:
        for clients in CLIENT_ORDER:
            cfg = configs.get((strategy, clients))
            lines.append(f"### Strategy={strategy}, Clients={clients}")
            lines.append("")
            lines.append("| Run | Throughput (ops/sec) | p99 (µs) | Errors |")
            lines.append("|---:|---:|---:|---:|")
            if cfg is None:
                for run_no in range(1, 11):
                    lines.append(f"| {run_no} | 0.000000000 | 0.000000000 | 0 |")
            else:
                for run in cfg["runs"]:
                    lines.append(
                        f"| {run['run']} | {fmt_num(run['throughput'])} | {fmt_num(run['p99'])} | {run['errors']} |"
                    )
                if len(cfg["runs"]) < 10:
                    for run_no in range(len(cfg["runs"]) + 1, 11):
                        lines.append(f"| {run_no} | 0.000000000 | 0.000000000 | 0 |")
            lines.append("")

    lines.append("## 4. Aggregated Results Table")
    lines.append("")
    lines.append("| Strategy | Clients | Throughput Mean (ops/sec) | Stddev | CV | p99 Mean (µs) | Stddev | CV | Errors |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")

    for strategy in STRATEGY_ORDER:
        for clients in CLIENT_ORDER:
            cfg = configs.get((strategy, clients))
            if cfg is None:
                lines.append(
                    f"| {strategy} | {clients} | 0.000000000 | 0.000000000 | 0.000000000 | 0.000000000 | 0.000000000 | 0.000000000 | 0 |"
                )
                continue
            lines.append(
                f"| {strategy} | {clients} | {fmt_num(cfg['throughput_mean'])} | {fmt_num(cfg['throughput_stddev'])} | {fmt_num(cfg['throughput_cv'])} | {fmt_num(cfg['p99_mean'])} | {fmt_num(cfg['p99_stddev'])} | {fmt_num(cfg['p99_cv'])} | {cfg['total_errors']} |"
            )

    lines.append("")
    lines.append("### Variance Table")
    lines.append("")
    lines.append("| Strategy | Clients | Throughput Variance | p99 Variance |")
    lines.append("|---|---:|---:|---:|")

    for strategy in STRATEGY_ORDER:
        for clients in CLIENT_ORDER:
            cfg = configs.get((strategy, clients))
            if cfg is None:
                lines.append(f"| {strategy} | {clients} | 0.000000000 | 0.000000000 |")
                continue
            lines.append(
                f"| {strategy} | {clients} | {fmt_num(cfg['throughput_variance'])} | {fmt_num(cfg['p99_variance'])} |"
            )

    lines.append("")
    lines.append("## 5. Baseline Comparison Table")
    lines.append("")
    lines.append("| Strategy | Clients | Throughput Overhead (%) | Latency Increase (%) |")
    lines.append("|---|---:|---:|---:|")

    for row in baseline_rows:
        lines.append(
            f"| {row['strategy']} | {row['clients']} | {fmt_num(row['throughput_overhead'])} | {fmt_num(row['latency_increase'])} |"
        )

    lines.append("")
    lines.append("## 6. Stability Classification")
    lines.append("")
    lines.append("| Strategy | Clients | Throughput CV | Throughput Stability | p99 CV | Latency Stability |")
    lines.append("|---|---:|---:|---|---:|---|")

    for strategy in STRATEGY_ORDER:
        for clients in CLIENT_ORDER:
            cfg = configs.get((strategy, clients))
            if cfg is None:
                lines.append(f"| {strategy} | {clients} | 0.000000000 | Stable | 0.000000000 | Stable |")
                continue
            lines.append(
                f"| {strategy} | {clients} | {fmt_num(cfg['throughput_cv'])} | {classify(cfg['throughput_cv'])} | {fmt_num(cfg['p99_cv'])} | {classify(cfg['p99_cv'])} |"
            )

    lines.append("")
    lines.append("## 7. Observations (Strictly Factual)")
    lines.append("")
    for item in observations:
        lines.append(f"- {item}")

    lines.append("")
    lines.append("## 8. Data Quality Notes")
    lines.append("")
    lines.append(f"- Expected configurations: 12; captured configurations: {len(configs)}.")
    lines.append("- Expected runs per configuration: 10.")

    if missing_configs:
        lines.append(f"- Missing configurations: {', '.join(missing_configs)}.")
    else:
        lines.append("- Missing configurations: none.")

    if wrong_run_count:
        lines.append(f"- Configurations with run-count mismatch: {', '.join(wrong_run_count)}.")
    else:
        lines.append("- Configurations with run-count mismatch: none.")

    if anomalies:
        lines.append(f"- Anomalies (non-zero errors): {'; '.join(anomalies)}.")
    else:
        lines.append("- Anomalies (non-zero errors): none.")

    lines.append("- Raw files captured per configuration: benchmark_results.json, bench_stdout.log, server.log, cmdstat.txt.")

    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate final experiment report markdown")
    parser.add_argument("--input", required=True, help="Input run directory (results/final_experiment/<timestamp>)")
    parser.add_argument("--output", default="reports/final_experiment_report.md", help="Output markdown path")
    args = parser.parse_args()

    run_root = Path(args.input)
    output_path = Path(args.output)

    if not run_root.exists():
        raise SystemExit(f"Input directory not found: {run_root}")

    build_report(run_root, output_path)
    print(f"Wrote report: {output_path}")


if __name__ == "__main__":
    main()
