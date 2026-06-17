#!/usr/bin/env python3
"""Summarize Mac M2 research benchmark outputs into a paper-ready CSV."""

from __future__ import annotations

import argparse
import csv
import json
import re
from pathlib import Path
from typing import Dict, Tuple


def parse_cmdstat(path: Path) -> Tuple[float, int]:
    if not path.exists():
        return 0.0, 0

    text = path.read_text(encoding="utf-8", errors="ignore")

    lock_wait_match = re.search(r"cmdstat_lock_wait_us:(\d+)", text)
    lock_wait_us = int(lock_wait_match.group(1)) if lock_wait_match else 0

    total_time_us = 0
    for m in re.finditer(r"total_time_us=(\d+)", text):
        total_time_us += int(m.group(1))

    if total_time_us <= 0:
        return 0.0, lock_wait_us

    contention_pct = (lock_wait_us / total_time_us) * 100.0
    return contention_pct, lock_wait_us


def load_machine_details(path: Path) -> Dict[str, str]:
    details: Dict[str, str] = {}
    if not path.exists():
        return details

    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            details[k.strip()] = v.strip()
    return details


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, help="Root run directory under results/macos_m2/")
    parser.add_argument("--output", required=True, help="CSV output path")
    args = parser.parse_args()

    root = Path(args.input)
    out_path = Path(args.output)

    machine = load_machine_details(root / "machine_details.txt")

    rows = []
    for core_dir in sorted(root.glob("core_*")):
        core_label = core_dir.name.replace("core_", "")

        for strategy_dir in sorted(core_dir.iterdir()):
            if not strategy_dir.is_dir():
                continue

            bench_json = strategy_dir / "benchmark_results.json"
            if not bench_json.exists():
                continue

            with bench_json.open("r", encoding="utf-8") as f:
                data = json.load(f)

            contention_pct, lock_wait_us = parse_cmdstat(strategy_dir / "cmdstat.txt")

            for r in data.get("results", []):
                # Filter to mixed workload rows only (the default experiment).
                if r.get("name") != "mixed":
                    continue

                rows.append(
                    {
                        "core_config": core_label,
                        "metrics_strategy": strategy_dir.name,
                        "concurrency": r.get("concurrency"),
                        "runs": r.get("runs"),
                        "throughput_ops_sec_mean": r.get("ops_per_sec_mean"),
                        "throughput_ops_sec_stddev": r.get("ops_per_sec_stddev"),
                        "p50_us_mean": r.get("p50_us_mean"),
                        "p50_us_stddev": r.get("p50_us_stddev"),
                        "p99_us_mean": r.get("p99_us_mean"),
                        "p99_us_stddev": r.get("p99_us_stddev"),
                        "contention_pct_estimate": round(contention_pct, 4),
                        "lock_wait_us": lock_wait_us,
                        "os": machine.get("os", ""),
                        "rustc": machine.get("rustc", ""),
                        "logicalcpu": machine.get("logicalcpu", ""),
                        "physicalcpu": machine.get("physicalcpu", ""),
                        "mem_bytes": machine.get("mem_bytes", ""),
                    }
                )

    out_path.parent.mkdir(parents=True, exist_ok=True)

    fieldnames = [
        "core_config",
        "metrics_strategy",
        "concurrency",
        "runs",
        "throughput_ops_sec_mean",
        "throughput_ops_sec_stddev",
        "p50_us_mean",
        "p50_us_stddev",
        "p99_us_mean",
        "p99_us_stddev",
        "contention_pct_estimate",
        "lock_wait_us",
        "os",
        "rustc",
        "logicalcpu",
        "physicalcpu",
        "mem_bytes",
    ]

    with out_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Wrote {len(rows)} rows to {out_path}")


if __name__ == "__main__":
    main()
