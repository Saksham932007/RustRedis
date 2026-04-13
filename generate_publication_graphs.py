#!/usr/bin/env python3
"""Generate publication-quality benchmark graphs from final_experiment_report_enhanced.md."""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Tuple

import matplotlib.pyplot as plt

REPORT_PATH = Path("final_experiment_report_enhanced.md")

STRATEGY_ORDER = ["disabled", "global_mutex", "sharded", "thread_local"]
CONCURRENCY_ORDER = [100, 500, 1000]


def parse_aggregated_rows(report_text: str) -> Dict[Tuple[str, int], dict]:
    """Parse the aggregated metrics table keyed by (strategy, concurrency)."""
    rows: Dict[Tuple[str, int], dict] = {}

    in_aggregated = False
    for raw_line in report_text.splitlines():
        line = raw_line.strip()

        if line.startswith("## 4. Aggregated Results Table"):
            in_aggregated = True
            continue

        if in_aggregated and line.startswith("### Variance Table"):
            break

        if not in_aggregated:
            continue

        if not line.startswith("|"):
            continue

        cells = [cell.strip() for cell in line.split("|")[1:-1]]
        if len(cells) != 9:
            continue

        if cells[0].lower() in {"strategy", "------------"}:
            continue

        strategy = cells[0]
        concurrency = int(cells[1])

        rows[(strategy, concurrency)] = {
            "strategy": strategy,
            "concurrency": concurrency,
            "throughput_mean": float(cells[2]),
            "throughput_stddev": float(cells[3]),
            "throughput_cv": float(cells[4]),
            "p99_mean": float(cells[5]),
            "p99_stddev": float(cells[6]),
            "p99_cv": float(cells[7]),
        }

    return rows


def parse_raw_runs(report_text: str) -> Dict[Tuple[str, int], dict]:
    """Parse per-run throughput and p99 values from raw results sections."""
    raw_data: Dict[Tuple[str, int], dict] = defaultdict(lambda: {"throughput": [], "p99": []})

    lines = report_text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i].strip()

        if line.startswith("### Strategy=") and ", Clients=" in line:
            header = line.removeprefix("### Strategy=")
            strategy, clients_str = header.split(", Clients=")
            concurrency = int(clients_str)

            i += 1
            while i < len(lines) and not lines[i].strip().startswith("| Run"):
                i += 1

            # Skip header and separator rows
            i += 2

            while i < len(lines):
                row = lines[i].strip()
                if not row.startswith("|"):
                    break

                cells = [cell.strip() for cell in row.split("|")[1:-1]]
                if len(cells) != 4:
                    break

                try:
                    throughput = float(cells[1])
                    p99 = float(cells[2])
                except ValueError:
                    break

                raw_data[(strategy, concurrency)]["throughput"].append(throughput)
                raw_data[(strategy, concurrency)]["p99"].append(p99)
                i += 1

            continue

        i += 1

    return raw_data


def build_dataset(report_text: str) -> List[dict]:
    """Build a normalized dataset combining aggregate metrics and raw runs."""
    aggregated = parse_aggregated_rows(report_text)
    raw = parse_raw_runs(report_text)

    dataset: List[dict] = []
    for strategy in STRATEGY_ORDER:
        for concurrency in CONCURRENCY_ORDER:
            key = (strategy, concurrency)
            if key not in aggregated:
                raise ValueError(f"Missing aggregated row for {strategy}/{concurrency}")
            if key not in raw or len(raw[key]["throughput"]) == 0:
                raise ValueError(f"Missing raw runs for {strategy}/{concurrency}")

            record = dict(aggregated[key])
            record["raw_throughput"] = raw[key]["throughput"]
            record["raw_p99"] = raw[key]["p99"]
            dataset.append(record)

    return dataset


def group_by_strategy(dataset: List[dict]) -> Dict[str, List[dict]]:
    grouped: Dict[str, List[dict]] = {s: [] for s in STRATEGY_ORDER}
    for record in dataset:
        grouped[record["strategy"]].append(record)

    for strategy in grouped:
        grouped[strategy].sort(key=lambda row: row["concurrency"])

    return grouped


def make_throughput_plot(grouped: Dict[str, List[dict]]) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))

    for strategy in STRATEGY_ORDER:
        rows = grouped[strategy]
        x = [r["concurrency"] for r in rows]
        y = [r["throughput_mean"] for r in rows]
        yerr = [r["throughput_stddev"] for r in rows]
        ax.errorbar(x, y, yerr=yerr, marker="o", linewidth=1.8, capsize=4, label=strategy)

    ax.set_title("Throughput vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("Throughput (ops/sec)")
    ax.grid(True, alpha=0.25)
    ax.legend(loc="upper right")

    fig.tight_layout()
    fig.savefig("throughput_vs_concurrency.png", dpi=300)
    plt.close(fig)


def make_latency_plot(grouped: Dict[str, List[dict]]) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))

    for strategy in STRATEGY_ORDER:
        rows = grouped[strategy]
        x = [r["concurrency"] for r in rows]
        y = [r["p99_mean"] for r in rows]
        yerr = [r["p99_stddev"] for r in rows]
        ax.errorbar(x, y, yerr=yerr, marker="o", linewidth=1.8, capsize=4, label=strategy)

    ax.set_title("Tail Latency (p99) vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("p99 Latency (µs)")
    ax.grid(True, alpha=0.25)
    ax.legend(loc="upper right")

    fig.tight_layout()
    fig.savefig("latency_vs_concurrency.png", dpi=300)
    plt.close(fig)


def make_cv_plot(grouped: Dict[str, List[dict]]) -> None:
    fig, ax = plt.subplots(figsize=(8, 5))

    for strategy in STRATEGY_ORDER:
        rows = grouped[strategy]
        x = [r["concurrency"] for r in rows]
        y = [r["throughput_cv"] for r in rows]
        ax.plot(x, y, marker="o", linewidth=1.8, label=strategy)

    ax.axhline(0.1, linestyle="--", linewidth=1.2, label="CV=0.1 stable")
    ax.axhline(0.3, linestyle=":", linewidth=1.2, label="CV=0.3 unstable threshold")

    ax.set_title("Performance Variability (CV) vs Concurrency")
    ax.set_xlabel("Concurrency (clients)")
    ax.set_ylabel("Coefficient of Variation (CV)")
    ax.grid(True, alpha=0.25)
    ax.legend(loc="upper right")

    fig.tight_layout()
    fig.savefig("cv_vs_concurrency.png", dpi=300)
    plt.close(fig)


def make_throughput_boxplot(grouped: Dict[str, List[dict]]) -> None:
    fig, ax = plt.subplots(figsize=(12.5, 6.5))

    box_data: List[List[float]] = []
    labels: List[str] = []
    for strategy in STRATEGY_ORDER:
        for concurrency in CONCURRENCY_ORDER:
            row = next(r for r in grouped[strategy] if r["concurrency"] == concurrency)
            box_data.append(row["raw_throughput"])
            labels.append(f"{strategy}\n{concurrency}")

    ax.boxplot(box_data, tick_labels=labels)
    ax.set_title("Throughput Distribution Across Runs")
    ax.set_xlabel("Strategy + Concurrency")
    ax.set_ylabel("Throughput (ops/sec)")
    plt.setp(ax.get_xticklabels(), rotation=35, ha="right")
    ax.grid(True, axis="y", alpha=0.25)

    fig.tight_layout()
    fig.savefig("throughput_distribution_boxplot.png", dpi=300)
    plt.close(fig)


def main() -> None:
    if not REPORT_PATH.exists():
        raise FileNotFoundError(f"Input report not found: {REPORT_PATH}")

    report_text = REPORT_PATH.read_text(encoding="utf-8")
    dataset = build_dataset(report_text)
    grouped = group_by_strategy(dataset)

    make_throughput_plot(grouped)
    make_latency_plot(grouped)
    make_cv_plot(grouped)
    make_throughput_boxplot(grouped)

    print("Generated files:")
    print("- throughput_vs_concurrency.png")
    print("- latency_vs_concurrency.png")
    print("- cv_vs_concurrency.png")
    print("- throughput_distribution_boxplot.png")


if __name__ == "__main__":
    main()
