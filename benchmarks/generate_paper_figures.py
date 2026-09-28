#!/usr/bin/env python3
"""Generate the two paper figures directly from rmit_analysis.json data.

Figure 1: overhead % vs concurrency, 5 strategies, advanced dataset
(averaged over the 3 workloads at each concurrency level) -> figures/fig1_overhead_vs_concurrency.png

Figure 2: overhead % by strategy, grouped by dataset (laptop / Azure D4s_v6 /
Azure D8s_v6 advanced) -> figures/fig2_cross_dataset_ranking.png

Every number plotted here is recomputed from raw_data_rmit.csv / rmit_analysis.json,
never hand-copied from the paper's tables, so the figures and the tables are two
views of the same source data and cannot silently drift apart.
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path
from statistics import mean

import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parent.parent
FIG_DIR = ROOT / "figures"
FIG_DIR.mkdir(exist_ok=True)

# Fixed categorical order/colors (validated colorblind-safe palette, slots 1-5).
STRATEGIES = ["thread_local", "sharded_2key", "global_mutex", "hdr_histogram", "sharded_n"]
LABELS = {
    "thread_local": "ThreadLocal",
    "sharded_2key": "Sharded-2key",
    "global_mutex": "GlobalMutex",
    "hdr_histogram": "HdrHistogram",
    "sharded_n": "Sharded-N",
}
COLORS = {
    "thread_local": "#2a78d6",   # slot 1 blue
    "sharded_2key": "#eb6834",   # slot 2 orange
    "global_mutex": "#1baf7a",   # slot 3 aqua
    "hdr_histogram": "#eda100",  # slot 4 yellow
    "sharded_n": "#e87ba4",      # slot 5 magenta
}
# Redundant encoding for grayscale/B&W print (line style + marker shape, not color alone).
LINESTYLES = {
    "thread_local": ("-", "o"),
    "sharded_2key": ("--", "s"),
    "global_mutex": ("-.", "^"),
    "hdr_histogram": (":", "D"),
    "sharded_n": ((0, (3, 1, 1, 1)), "v"),
}
HATCHES = {
    "thread_local": "",
    "sharded_2key": "//",
    "global_mutex": "xx",
    "hdr_histogram": "..",
    "sharded_n": "\\\\",
}

plt.rcParams.update(
    {
        "figure.facecolor": "white",
        "axes.facecolor": "white",
        "savefig.facecolor": "white",
        "axes.edgecolor": "#898781",
        "axes.labelcolor": "#0b0b0b",
        "text.color": "#0b0b0b",
        "xtick.color": "#52514e",
        "ytick.color": "#52514e",
        "axes.grid": True,
        "grid.color": "#e1e0d9",
        "grid.linewidth": 0.8,
        "font.size": 11,
        "font.family": "sans-serif",
    }
)


def load(path: str) -> list[dict]:
    return json.load(open(ROOT / path))["paired_ratio_vs_baseline"]


def overhead_pct(rec: dict) -> float:
    # Per-cell point estimate is the paired-block RATIO MEDIAN, matching
    # analyze_rmit_results.py's own convention (medians, not means, are the
    # primary point estimate throughout this project - means get pulled
    # around by outliers exactly in the runs a slow block would land on).
    # The paper's tables report the arithmetic mean of these per-cell
    # medians across concurrency/workload cells ("mean overhead" = mean of
    # medians, not mean of means) - verified by reproducing §4's laptop
    # thread_local figure (0.99%) and §4b's mixed-workload thread_local
    # figure (0.44%) from this exact computation before trusting it here.
    return (1.0 - rec["ratio_median"]) * 100.0


def figure1_overhead_vs_concurrency() -> None:
    records = load("experiment_results_rmit_advanced/rmit_analysis.json")
    # group by (strategy, concurrency), average overhead across the 3 workloads
    by_strategy_conc: dict[tuple[str, int], list[float]] = defaultdict(list)
    for r in records:
        by_strategy_conc[(r["strategy"], r["concurrency"])].append(overhead_pct(r))

    concurrencies = sorted({c for (_, c) in by_strategy_conc})

    fig, ax = plt.subplots(figsize=(7.5, 4.8), dpi=200)
    for strat in STRATEGIES:
        ys = [mean(by_strategy_conc[(strat, c)]) for c in concurrencies]
        ls, marker = LINESTYLES[strat]
        ax.plot(
            concurrencies,
            ys,
            color=COLORS[strat],
            linestyle=ls,
            marker=marker,
            markersize=6,
            linewidth=2,
            label=LABELS[strat],
        )

    ax.set_xlabel("Concurrent clients")
    ax.set_ylabel("Mean overhead vs. disabled (%)")
    ax.set_title(
        "Instrumentation overhead stays flat and small up to 3000 clients\n"
        "(Azure D8s_v6, averaged over mixed/read-heavy/write-heavy workloads)",
        fontsize=10.5,
    )
    ax.set_xscale("log")
    ax.set_xticks(concurrencies)
    ax.set_xticklabels([str(c) for c in concurrencies])
    ax.set_ylim(bottom=0)
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.legend(frameon=False, loc="upper left", ncol=2, fontsize=9)
    fig.tight_layout()
    out = FIG_DIR / "fig1_overhead_vs_concurrency.png"
    fig.savefig(out)
    print(f"wrote {out}")


def figure2_cross_dataset_ranking() -> None:
    datasets = [
        ("Laptop\n(i3-10110U)", "experiment_results_rmit/rmit_analysis.json"),
        ("Azure D4s_v6", "experiment_results_rmit_azure/rmit_analysis.json"),
        ("Azure D8s_v6\n(advanced)", "experiment_results_rmit_advanced/rmit_analysis.json"),
    ]

    dataset_overhead: dict[str, dict[str, float]] = {}
    for label, path in datasets:
        records = load(path)
        by_strategy: dict[str, list[float]] = defaultdict(list)
        for r in records:
            by_strategy[r["strategy"]].append(overhead_pct(r))
        dataset_overhead[label] = {s: mean(vs) for s, vs in by_strategy.items()}

    n_groups = len(datasets)
    n_bars = len(STRATEGIES)
    bar_width = 0.15
    x = range(n_groups)

    fig, ax = plt.subplots(figsize=(7.5, 4.8), dpi=200)
    for i, strat in enumerate(STRATEGIES):
        offsets = [xi + (i - (n_bars - 1) / 2) * bar_width for xi in x]
        heights = [dataset_overhead[label][strat] for label, _ in datasets]
        ax.bar(
            offsets,
            heights,
            width=bar_width,
            color=COLORS[strat],
            hatch=HATCHES[strat],
            edgecolor="white",
            linewidth=0.6,
            label=LABELS[strat],
        )

    ax.set_xticks(list(x))
    ax.set_xticklabels([label for label, _ in datasets])
    ax.set_ylabel("Mean overhead vs. disabled (%)")
    ax.set_title(
        "thread_local ranks cheapest and sharded_2key ranks 2nd in all three\n"
        "independent datasets; ranks 3-5 reshuffle between machines",
        fontsize=10.5,
    )
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.legend(frameon=False, loc="upper left", ncol=2, fontsize=9)
    fig.tight_layout()
    out = FIG_DIR / "fig2_cross_dataset_ranking.png"
    fig.savefig(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    figure1_overhead_vs_concurrency()
    figure2_cross_dataset_ranking()
