#!/usr/bin/env python3
"""Cluster-aware minimum detectable effect for the pooled 11-run D8s_v6 dataset.

The pooled dataset (experiments/azure_d8s_v6/pooled) has n=165 paired-block
ratios per (strategy, concurrency, workload) cell, but those 165 are not
i.i.d.: they come from 11 independent runs (11 clusters of 15 correlated
within-run blocks each - 10 from the batch of separately-provisioned VMs
plus the original single validation run pooled in as an 11th replicate;
see combine_iterations.py). scripts/compute_mde.py treats all 165 as
independent, which is optimistic if there is real between-run variance.

This script computes a conservative bound instead: for each cell, take the
11 per-run median ratios as the unit of analysis (one number per run), then
pool the resulting between-run SD across all cells and divide by sqrt(11)
rather than sqrt(165). This is the same MDE formula as compute_mde.py,
applied one level up the clustering hierarchy - it answers "how small an
effect can we detect if we only trust one number per run", which is the
right question when the goal is a result that generalizes across VM
instances/regions rather than one specific run's own noise.

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
N_CLUSTERS = 11


def main() -> None:
    path = ROOT / "experiments/azure_d8s_v6/pooled" / "raw_data_rmit.csv"
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

    print(f"cells with {N_CLUSTERS} per-run medians: "
          f"{sum(1 for v in per_cell_iter_medians.values() if len(v) == N_CLUSTERS)}"
          f" / {len(per_cell_iter_medians)}")
    print(f"between-run SD of per-cell median ratios (pooled across cells): {sd_between_vm:.4f}")
    print(f"cluster-aware MDE (n={N_CLUSTERS} runs, treating each run's median as one point): {mde_clustered:.2f}%")


if __name__ == "__main__":
    main()
