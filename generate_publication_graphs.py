#!/usr/bin/env python3
"""Generate publication-quality benchmark graphs from JSON or summary-table reports."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from statistics import mean, stdev
from typing import Dict, Iterable, List, Tuple

import matplotlib.pyplot as plt

plt.style.use("default")
plt.rcParams.update(
    {
        "figure.facecolor": "white",
        "axes.facecolor": "white",
        "savefig.facecolor": "white",
    }
)

DEFAULT_INPUT_CANDIDATES = [
    Path("reports/final_experiment_report_enhanced.md"),
    Path("reports/final_experiment_report.md"),
    Path("results/benchmark_results.json"),
    Path("benchmarks/results/benchmark_results.json"),
]

STRATEGY_ORDER = ["disabled", "global_mutex", "sharded", "thread_local"]
LEGEND_LABELS = {
    "disabled": "Disabled",
    "global_mutex": "GlobalMutex",
    "sharded": "Sharded",
    "thread_local": "ThreadLocal",
}

# Publication-style controls required by the prompt.
FIG_SIZE = (8, 5)
TITLE_SIZE = 16
AXIS_LABEL_SIZE = 14
TICK_LABEL_SIZE = 12
LEGEND_SIZE = 12
LINE_WIDTH = 2.5
MARKER_SIZE = 6
GRID_ALPHA = 0.15
MARKERS = ["o", "s", "^", "D"]
STRATEGY_COLORS = {
    "disabled": "#1f77b4",
    "global_mutex": "#ff7f0e",
    "sharded": "#2ca02c",
    "thread_local": "#d62728",
}


def parse_aggregated_rows(report_text: str) -> Dict[Tuple[str, int], dict]:
    """Parse the aggregated metrics table keyed by (strategy, concurrency)."""
    rows: Dict[Tuple[str, int], dict] = {}

    in_aggregated = False
    for raw_line in report_text.splitlines():
        line = raw_line.strip()

        if line.startswith("## 4. Aggregated Results Table"):
            in_aggregated = True
            continue

        if in_aggregated and line.startswith("###"):
            break

        if not in_aggregated or not line.startswith("|"):
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
            "runs": 0,
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


def build_dataset_from_markdown(path: Path) -> List[dict]:
    report_text = path.read_text(encoding="utf-8")
    aggregated = parse_aggregated_rows(report_text)
    raw = parse_raw_runs(report_text)

    dataset: List[dict] = []
    for key, agg in aggregated.items():
        row = dict(agg)
        row["raw_throughput"] = raw.get(key, {}).get("throughput", [])
        row["raw_p99"] = raw.get(key, {}).get("p99", [])
        row["runs"] = len(row["raw_throughput"])
        dataset.append(row)

    if not dataset:
        raise ValueError(f"No aggregated strategy data found in markdown file: {path}")

    return dataset


def _first_numeric(mapping: dict, keys: Iterable[str]) -> float | None:
    for key in keys:
        value = mapping.get(key)
        if value is None:
            continue
        try:
            return float(value)
        except (TypeError, ValueError):
            continue
    return None


def _extract_numeric_list(rows: Iterable[dict], keys: Iterable[str]) -> List[float]:
    values: List[float] = []
    for row in rows:
        numeric = _first_numeric(row, keys)
        if numeric is not None:
            values.append(numeric)
    return values


def _infer_strategy(row: dict) -> str | None:
    strategy = row.get("strategy")
    if isinstance(strategy, str):
        return strategy

    target = row.get("target")
    if isinstance(target, str) and target in STRATEGY_ORDER:
        return target

    name = row.get("name")
    if isinstance(name, str):
        lower_name = name.lower()
        for known_strategy in STRATEGY_ORDER:
            if known_strategy in lower_name:
                return known_strategy

    return None


def build_dataset_from_json(path: Path) -> List[dict]:
    payload = json.loads(path.read_text(encoding="utf-8"))

    if isinstance(payload, dict) and isinstance(payload.get("results"), list):
        records = payload["results"]
    elif isinstance(payload, list):
        records = payload
    else:
        raise ValueError(f"Unsupported JSON schema for plotting: {path}")

    dataset: List[dict] = []
    for record in records:
        if not isinstance(record, dict):
            continue

        strategy = _infer_strategy(record)
        if not strategy:
            continue

        concurrency = record.get("concurrency", record.get("clients"))
        if concurrency is None:
            continue

        per_run = record.get("per_run") if isinstance(record.get("per_run"), list) else []
        raw_throughput = _extract_numeric_list(per_run, ["ops_per_sec", "throughput"])
        raw_p99 = _extract_numeric_list(per_run, ["p99_us", "p99"])

        throughput_mean = _first_numeric(record, ["throughput_mean", "ops_per_sec_mean", "ops_per_sec"])
        if throughput_mean is None and raw_throughput:
            throughput_mean = mean(raw_throughput)

        p99_mean = _first_numeric(record, ["p99_mean", "p99_us_mean", "p99_us"])
        if p99_mean is None and raw_p99:
            p99_mean = mean(raw_p99)

        if throughput_mean is None or p99_mean is None:
            continue

        throughput_stddev = _first_numeric(record, ["throughput_stddev", "ops_per_sec_stddev"])
        if throughput_stddev is None and len(raw_throughput) >= 2:
            throughput_stddev = stdev(raw_throughput)
        if throughput_stddev is None:
            throughput_stddev = 0.0

        p99_stddev = _first_numeric(record, ["p99_stddev", "p99_us_stddev"])
        if p99_stddev is None and len(raw_p99) >= 2:
            p99_stddev = stdev(raw_p99)
        if p99_stddev is None:
            p99_stddev = 0.0

        throughput_cv = _first_numeric(record, ["throughput_cv"]) 
        if throughput_cv is None:
            throughput_cv = (throughput_stddev / throughput_mean) if throughput_mean > 0 else 0.0

        p99_cv = _first_numeric(record, ["p99_cv"]) 
        if p99_cv is None:
            p99_cv = (p99_stddev / p99_mean) if p99_mean > 0 else 0.0

        runs = int(record.get("runs") or len(raw_throughput) or 0)

        dataset.append(
            {
                "strategy": strategy,
                "concurrency": int(concurrency),
                "throughput_mean": float(throughput_mean),
                "throughput_stddev": float(throughput_stddev),
                "throughput_cv": float(throughput_cv),
                "p99_mean": float(p99_mean),
                "p99_stddev": float(p99_stddev),
                "p99_cv": float(p99_cv),
                "raw_throughput": raw_throughput,
                "raw_p99": raw_p99,
                "runs": runs,
            }
        )

    if not dataset:
        raise ValueError(
            "No strategy-level records were detected in JSON input. "
            "Use a final-experiment strategy JSON/report with strategy names."
        )

    return dataset


def resolve_input_path(explicit_input: Path | None) -> Path:
    if explicit_input is not None:
        if not explicit_input.exists():
            raise FileNotFoundError(f"Input file not found: {explicit_input}")
        return explicit_input

    for candidate in DEFAULT_INPUT_CANDIDATES:
        if candidate.exists():
            return candidate

    candidates_text = ", ".join(str(path) for path in DEFAULT_INPUT_CANDIDATES)
    raise FileNotFoundError(f"No default input file found. Checked: {candidates_text}")


def load_dataset(input_path: Path) -> List[dict]:
    if input_path.suffix.lower() == ".json":
        return build_dataset_from_json(input_path)

    if input_path.suffix.lower() in {".md", ".markdown"}:
        return build_dataset_from_markdown(input_path)

    # Try markdown parser first, then JSON parser as a fallback.
    try:
        return build_dataset_from_markdown(input_path)
    except Exception:
        return build_dataset_from_json(input_path)


def sorted_strategies(dataset: List[dict]) -> List[str]:
    discovered = {row["strategy"] for row in dataset}
    ordered = [s for s in STRATEGY_ORDER if s in discovered]
    ordered.extend(sorted(s for s in discovered if s not in STRATEGY_ORDER))
    return ordered


def group_by_strategy(dataset: List[dict]) -> Dict[str, List[dict]]:
    grouped: Dict[str, List[dict]] = defaultdict(list)
    for row in dataset:
        grouped[row["strategy"]].append(row)

    for strategy in grouped:
        grouped[strategy].sort(key=lambda entry: entry["concurrency"])

    return grouped


def style_axis(ax: plt.Axes, title: str, ylabel: str, xlabel: str = "Concurrency (clients)") -> None:
    ax.set_title(title, fontsize=TITLE_SIZE)
    ax.set_xlabel(xlabel, fontsize=AXIS_LABEL_SIZE)
    ax.set_ylabel(ylabel, fontsize=AXIS_LABEL_SIZE)
    ax.tick_params(axis="both", labelsize=TICK_LABEL_SIZE)
    ax.grid(False)
    ax.grid(axis="y", alpha=GRID_ALPHA, color="#bfbfbf", linewidth=0.8)


def confidence_interval_95(row: dict) -> float:
    raw = row.get("raw_throughput", [])
    if len(raw) >= 2:
        sd = stdev(raw)
        return 1.96 * sd / math.sqrt(len(raw))

    runs = int(row.get("runs", 0))
    if runs >= 2:
        return 1.96 * float(row.get("throughput_stddev", 0.0)) / math.sqrt(runs)

    return 0.0


def make_throughput_plot(grouped: Dict[str, List[dict]], strategies: List[str], output_dir: Path) -> Path:
    fig, ax = plt.subplots(figsize=FIG_SIZE)

    for idx, strategy in enumerate(strategies):
        rows = grouped[strategy]
        x = [row["concurrency"] for row in rows]
        y = [row["throughput_mean"] for row in rows]
        yerr = [confidence_interval_95(row) for row in rows]

        ax.errorbar(
            x,
            y,
            yerr=yerr,
            label=LEGEND_LABELS.get(strategy, strategy),
            color=STRATEGY_COLORS.get(strategy),
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=LINE_WIDTH,
            markersize=MARKER_SIZE,
            capsize=4,
            capthick=1.0,
            elinewidth=1.4,
        )

    style_axis(ax, "Throughput vs Concurrency", "Throughput (ops/sec)")
    ax.legend(loc="upper right", fontsize=LEGEND_SIZE, frameon=False)

    fig.tight_layout()
    output_path = output_dir / "throughput_vs_concurrency.png"
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    return output_path


def make_latency_plot(grouped: Dict[str, List[dict]], strategies: List[str], output_dir: Path) -> Path:
    fig, ax = plt.subplots(figsize=FIG_SIZE)

    for idx, strategy in enumerate(strategies):
        rows = grouped[strategy]
        x = [row["concurrency"] for row in rows]
        y = [row["p99_mean"] for row in rows]

        ax.plot(
            x,
            y,
            label=LEGEND_LABELS.get(strategy, strategy),
            color=STRATEGY_COLORS.get(strategy),
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=LINE_WIDTH,
            markersize=MARKER_SIZE,
        )

    style_axis(ax, "p99 Latency vs Concurrency", "p99 Latency (µs)")
    plt.ylim(bottom=0)
    ax.legend(loc="upper left", fontsize=LEGEND_SIZE, frameon=False)

    fig.tight_layout()
    output_path = output_dir / "latency_vs_concurrency.png"
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    return output_path


def make_cv_plot(grouped: Dict[str, List[dict]], strategies: List[str], output_dir: Path) -> Path:
    fig, ax = plt.subplots(figsize=FIG_SIZE)

    for idx, strategy in enumerate(strategies):
        rows = grouped[strategy]
        x = [row["concurrency"] for row in rows]
        y = [row["throughput_cv"] for row in rows]

        ax.plot(
            x,
            y,
            label=LEGEND_LABELS.get(strategy, strategy),
            color=STRATEGY_COLORS.get(strategy),
            marker=MARKERS[idx % len(MARKERS)],
            linewidth=LINE_WIDTH,
            markersize=MARKER_SIZE,
        )

    ax.axhline(0.1, linestyle="--", linewidth=1.4, alpha=0.35, color="#666666", label="Stable (CV = 0.1)")
    ax.axhline(0.3, linestyle="--", linewidth=1.4, alpha=0.35, color="#666666", label="Unstable (CV = 0.3)")

    style_axis(ax, "Coefficient of Variation vs Concurrency", "Coefficient of Variation (CV)")
    ax.legend(loc="lower right", fontsize=LEGEND_SIZE, frameon=False)

    fig.tight_layout()
    output_path = output_dir / "cv_vs_concurrency.png"
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    return output_path


def make_throughput_boxplot(grouped: Dict[str, List[dict]], strategies: List[str], output_dir: Path) -> Path | None:
    box_data: List[List[float]] = []
    labels: List[str] = []
    colors: List[str | None] = []

    for strategy in strategies:
        strategy_values: List[float] = []
        for row in grouped[strategy]:
            strategy_values.extend(row.get("raw_throughput", []))

        if strategy_values:
            box_data.append(strategy_values)
            labels.append(strategy)
            colors.append(STRATEGY_COLORS.get(strategy))

    if not box_data:
        return None

    fig, ax = plt.subplots(figsize=FIG_SIZE)
    box = ax.boxplot(box_data, tick_labels=labels, patch_artist=True, showfliers=True, widths=0.55)

    for patch, color in zip(box["boxes"], colors):
        if color is not None:
            patch.set_facecolor(color)
            patch.set_edgecolor(color)
        patch.set_alpha(0.3)
        patch.set_linewidth(1.5)

    style_axis(ax, "Throughput Distribution by Strategy", "Throughput (ops/sec)", xlabel="Strategy")
    ax.tick_params(axis="x", labelsize=TICK_LABEL_SIZE)

    fig.tight_layout()
    output_path = output_dir / "throughput_distribution.png"
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    return output_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate publication-quality benchmark plots")
    parser.add_argument(
        "--input",
        type=Path,
        default=None,
        help="Path to input benchmark file (.json or .md summary table)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("figures/canonical"),
        help="Directory where plot PNG files will be saved",
    )
    parser.add_argument(
        "--skip-boxplot",
        action="store_true",
        help="Skip optional throughput distribution boxplot",
    )
    args = parser.parse_args()

    input_path = resolve_input_path(args.input)
    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    dataset = load_dataset(input_path)
    strategies = sorted_strategies(dataset)
    grouped = group_by_strategy(dataset)

    outputs = [
        make_throughput_plot(grouped, strategies, output_dir),
        make_latency_plot(grouped, strategies, output_dir),
        make_cv_plot(grouped, strategies, output_dir),
    ]

    if not args.skip_boxplot:
        boxplot_path = make_throughput_boxplot(grouped, strategies, output_dir)
        if boxplot_path is not None:
            outputs.append(boxplot_path)

    print(f"Input: {input_path}")
    print("Generated files:")
    for output in outputs:
        print(f"- {output}")


if __name__ == "__main__":
    main()
