#!/usr/bin/env python3
"""Cluster-aware minimum detectable effect for the pooled 10-VM D8s_v6 dataset.

The pooled dataset (experiment_results_adv_pooled) has n=150 paired-block
ratios per (strategy, concurrency, workload) cell, but those 150 are not
i.i.d.: they come from 10 independently-provisioned VMs (10 clusters of 15
correlated within-VM blocks each). benchmarks/compute_mde.py treats all 150
as independent, which is optimistic if there is real between-VM variance.

This script computes a conservative bound instead: for each cell, take the
10 per-VM-iteration median ratios as the unit of analysis (one number per
VM), then pool the resulting between-VM SD across all cells and divide by
sqrt(10) rather than sqrt(150). This is the same MDE formula as
compute_mde.py, applied one level up the clustering hierarchy - it answers
"how small an effect can we detect if we only trust one number per VM",
which is the right question when the goal is a result that generalizes
across VM instances/regions rather than one specific VM's own noise.

Also reports the ratio between the naive (pooled-n) and clustered MDE as a
rough design effect.
"""

from __future__ import annotations

import csv
from collections import defaultdict
from pathlib import Path
from statistics import median, stdev

ROOT = Path(__file__).resolve().parent.parent
Z_ALPHA_2 = 1.96
Z_POWER = 0.84
N_CLUSTERS = 10


def main() -> None:
    path = ROOT / "experiment_results_adv_pooled" / "raw_data_rmit.csv"
    with path.open(newline="") as fh:
        rows = list(csv.DictReader(fh))

    # block key includes source_iteration implicitly via the offset block_id,
    # but we need the iteration explicitly to group per-VM.
    blocks: dict[tuple, dict[str, float]] = defaultdict(dict)
    iter_of_block: dict[tuple, str] = {}
    for r in rows:
        key = (r["block_id"], r["concurrency"], r["workload"])
        blocks[key][r["strategy"]] = float(r["throughput"])
        iter_of_block[key] = r["source_iteration"]

    # per (iteration, strategy, concurrency, workload) -> list of paired ratios
    per_iter_cell_ratios: dict[tuple, list[float]] = defaultdict(list)
    for key, d in blocks.items():
        if "disabled" not in d:
            continue
        base = d["disabled"]
        it = iter_of_block[key]
        _, conc, wl = key
        for s, v in d.items():
            if s == "disabled":
                continue
            per_iter_cell_ratios[(it, s, conc, wl)].append(v / base)

    # collapse each (iteration, cell) to its median -> one number per VM per cell
    per_cell_iter_medians: dict[tuple, list[float]] = defaultdict(list)
    for (it, s, conc, wl), vals in per_iter_cell_ratios.items():
        per_cell_iter_medians[(s, conc, wl)].append(median(vals))

    # pool the between-VM SD across all cells (mirrors compute_mde.py's global pooling)
    all_iter_medians = [v for vals in per_cell_iter_medians.values() for v in vals]
    sd_between_vm = stdev(all_iter_medians)
    mde_clustered = (Z_ALPHA_2 + Z_POWER) * sd_between_vm / N_CLUSTERS**0.5 * 100.0

    print(f"cells with 10 per-VM medians: {sum(1 for v in per_cell_iter_medians.values() if len(v) == N_CLUSTERS)}"
          f" / {len(per_cell_iter_medians)}")
    print(f"between-VM SD of per-cell median ratios (pooled across cells): {sd_between_vm:.4f}")
    print(f"cluster-aware MDE (n=10 VMs, treating each VM's median as one point): {mde_clustered:.2f}%")


if __name__ == "__main__":
    main()
