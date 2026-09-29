#!/usr/bin/env python3
"""Pool the 10 independent advanced-design iterations into one raw_data_rmit.csv.

Each iteration (experiment_results_adv_iter_01 .. _10) is a full, independent
RMIT run (6 strategies x 8 concurrency x 3 workloads x 15 repetitions = 2160
runs) on its own Azure D8s_v6 VM, with its own random seed and its own
block_id numbering 1-15. Concatenating them naively would let block_id 3 from
iteration 1 collide with block_id 3 from iteration 7 in the paired-ratio
comparison (analyze_rmit_results.py groups by block_id), which would pair
runs from two different VMs/times as if they were the same repetition. This
script offsets each iteration's block_id by iteration_number * 100 before
concatenating, so every block stays scoped to the single VM run it actually
came from - the paired-block comparison remains valid, and n_paired_blocks
becomes 150 (10 iterations x 15 blocks) per (strategy, concurrency, workload)
cell instead of 15.

Usage: python3 benchmarks/combine_iterations.py
Writes experiment_results_adv_pooled/raw_data_rmit.csv (and metadata_pooled.json).
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ITER_DIRS = [ROOT / f"experiment_results_adv_iter_{n:02d}" for n in range(1, 11)]
OUT_DIR = ROOT / "experiment_results_adv_pooled"


def main() -> None:
    OUT_DIR.mkdir(exist_ok=True)
    all_rows: list[dict] = []
    fieldnames: list[str] | None = None
    per_iter_counts = {}

    for i, d in enumerate(ITER_DIRS, start=1):
        csv_path = d / "raw_data_rmit.csv"
        with csv_path.open(newline="", encoding="utf-8") as fh:
            reader = csv.DictReader(fh)
            if fieldnames is None:
                fieldnames = reader.fieldnames
            elif reader.fieldnames != fieldnames:
                raise SystemExit(f"{csv_path}: column mismatch vs first file")
            rows = list(reader)
        for r in rows:
            r["block_id"] = str(i * 100 + int(r["block_id"]))
            r["source_iteration"] = f"{i:02d}"
        all_rows.extend(rows)
        per_iter_counts[d.name] = len(rows)
        print(f"{d.name}: {len(rows)} rows (block_id offset {i*100})")

    out_fieldnames = list(fieldnames) + ["source_iteration"]
    out_csv = OUT_DIR / "raw_data_rmit.csv"
    with out_csv.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=out_fieldnames)
        writer.writeheader()
        writer.writerows(all_rows)

    meta = {
        "description": (
            "Pooled dataset: 10 independent RMIT advanced-design runs on 10 "
            "separately-provisioned Azure Standard_D8s_v6 VMs (4 regions: "
            "Central India x2, South India, West US 3), each with its own "
            "random seed. block_id offset by iteration*100 to keep paired-block "
            "comparisons scoped to the single VM run they came from."
        ),
        "n_iterations": len(ITER_DIRS),
        "rows_per_iteration": per_iter_counts,
        "total_rows": len(all_rows),
    }
    (OUT_DIR / "metadata_pooled.json").write_text(json.dumps(meta, indent=2))
    print(f"\nTotal rows: {len(all_rows)}")
    print(f"Written: {out_csv}")


if __name__ == "__main__":
    main()
