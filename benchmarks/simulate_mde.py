#!/usr/bin/env python3
"""Simulation-based power / minimum detectable overhead for the RMIT paired design.

Method (adapted from the injected-slowdown procedure in Laaber et al. 2019,
Empirical Software Engineering, sec. 5.4): for each (strategy, concurrency
[, workload]) cell, take its paired throughput ratios (strategy / disabled,
same repetition block), remove that cell's own effect by dividing by their
median (a noise-only null with the cell's real noise structure and n), then
for each candidate overhead x: resample n ratios with replacement, multiply
by (1 - x), and test whether the 95% percentile-bootstrap CI of the median
(the same interval analyze_rmit_results.py reports) excludes 1.0.
x = 0 gives the empirical false-positive rate of the test.

Reports, per dataset, pooled detection rate per x and the smallest x with
pooled power >= 0.80. Requires numpy.
"""

from __future__ import annotations

import csv
from collections import defaultdict
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
DATASETS = [
    ("laptop", "experiment_results_rmit/raw_data_rmit.csv"),
    ("azure_d4s_v6", "experiment_results_rmit_azure/raw_data_rmit.csv"),
    ("azure_d8s_v6_advanced", "experiment_results_rmit_advanced/raw_data_rmit.csv"),
]
GRID_PCT = [0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0]
SIMS = 400
BOOT = 400
SEED = 0


def cell_ratios(path: Path) -> dict[tuple, np.ndarray]:
    with path.open(newline="") as fh:
        rows = list(csv.DictReader(fh))
    has_w = "workload" in rows[0]
    blocks: dict[tuple, dict[str, float]] = defaultdict(dict)
    for r in rows:
        key = (r["block_id"], r["concurrency"]) + ((r["workload"],) if has_w else ())
        blocks[key][r["strategy"]] = float(r["throughput"])
    cells: dict[tuple, list[float]] = defaultdict(list)
    for key, d in blocks.items():
        if "disabled" not in d:
            continue
        for s, v in d.items():
            if s != "disabled":
                cells[(s,) + key[1:]].append(v / d["disabled"])
    return {k: np.array(v) for k, v in cells.items()}


def detect_rate(r0: np.ndarray, x: float, rng: np.random.Generator) -> float:
    n = len(r0)
    sims = r0[rng.integers(0, n, size=(SIMS, n))] * (1.0 - x)
    boots = np.median(
        sims[np.arange(SIMS)[:, None, None], rng.integers(0, n, size=(SIMS, BOOT, n))], axis=2
    )
    lo = np.percentile(boots, 2.5, axis=1)
    hi = np.percentile(boots, 97.5, axis=1)
    return float(np.mean((hi < 1.0) | (lo > 1.0)))


if __name__ == "__main__":
    rng = np.random.default_rng(SEED)
    print(f"sims={SIMS} bootstrap={BOOT} seed={SEED}")
    print(f"{'dataset':<24}" + "".join(f"{g:>7.2f}%" for g in GRID_PCT) + "   MDE@80%")
    for name, rel in DATASETS:
        cells = cell_ratios(ROOT / rel)
        power = np.zeros(len(GRID_PCT))
        for r in cells.values():
            r0 = r / np.median(r)
            for i, g in enumerate(GRID_PCT):
                power[i] += detect_rate(r0, g / 100.0, rng)
        power /= len(cells)
        mde = next((g for g, p in zip(GRID_PCT, power) if g > 0 and p >= 0.80), None)
        mde_s = f"{mde:.2f}%" if mde is not None else ">5%"
        print(f"{name:<24}" + "".join(f"{p:>8.2f}" for p in power) + f"   {mde_s}")
