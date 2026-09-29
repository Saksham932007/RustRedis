#!/usr/bin/env python3
"""Minimum detectable effect (MDE) for the RMIT paired-block design (paper §7).

For each dataset: pool the per-block paired throughput ratios
(strategy / disabled, same repetition block, same concurrency/workload),
take their sample standard deviation SD, and report

    MDE = (z_{1-alpha/2} + z_{power}) * SD / sqrt(n)   (alpha=0.05, power=0.80)

where n is the median number of repetitions per (strategy, concurrency
[, workload]) cell. A rough paired-design estimate, not an exact power
calculation.
"""

from __future__ import annotations

import csv
from collections import Counter, defaultdict
from pathlib import Path
from statistics import median, stdev

ROOT = Path(__file__).resolve().parent.parent
Z_ALPHA_2 = 1.96
Z_POWER = 0.84

DATASETS = [
    ("azure_d4s_v6", "experiment_results_rmit_azure/raw_data_rmit.csv"),
    ("azure_d8s_v6_advanced_1x", "experiment_results_rmit_advanced/raw_data_rmit.csv"),
    ("azure_d8s_v6_advanced_10x_pooled", "experiment_results_adv_pooled/raw_data_rmit.csv"),
]


def analyze(path: Path) -> tuple[float, float, int]:
    with path.open(newline="") as fh:
        rows = list(csv.DictReader(fh))
    has_workload = "workload" in rows[0]
    block_key = ["block_id", "concurrency"] + (["workload"] if has_workload else [])
    cell_key = ["strategy", "concurrency"] + (["workload"] if has_workload else [])

    blocks: dict[tuple, dict[str, float]] = defaultdict(dict)
    for r in rows:
        blocks[tuple(r[k] for k in block_key)][r["strategy"]] = float(r["throughput"])

    ratios = [
        v / d["disabled"]
        for d in blocks.values()
        if "disabled" in d
        for s, v in d.items()
        if s != "disabled"
    ]
    reps = int(median(Counter(tuple(r[k] for k in cell_key) for r in rows).values()))
    sd = stdev(ratios)
    mde = (Z_ALPHA_2 + Z_POWER) * sd / reps**0.5
    return sd, mde * 100.0, reps


if __name__ == "__main__":
    print(f"{'dataset':<24}{'pooled SD':>10}{'reps/cell':>11}{'MDE (%)':>9}")
    for name, rel in DATASETS:
        sd, mde_pct, reps = analyze(ROOT / rel)
        print(f"{name:<24}{sd:>10.4f}{reps:>11d}{mde_pct:>9.2f}")
