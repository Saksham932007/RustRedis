#!/usr/bin/env python3
"""Recompute the two-state evidence from the original fixed-order v12 dataset (paper §2).

Reads experiment_results_v12/raw_data.csv (rows are in execution order) and
the anomaly rerun of sharded_2key @ c=500. The 130,000 ops/s cut-off is a
labelling threshold, not a statistical test.
"""

from __future__ import annotations

import csv
from collections import defaultdict
from pathlib import Path
from statistics import median

ROOT = Path(__file__).resolve().parent.parent / "experiment_results_v12"
CUT = 130_000.0


def main() -> None:
    rows = list(csv.DictReader((ROOT / "raw_data.csv").open()))
    tp = [float(r["throughput"]) for r in rows]
    fast = [t for t in tp if t > CUT]
    slow = [t for t in tp if t <= CUT]
    print(f"runs={len(tp)}  fast(>{CUT:.0f})={len(fast)} median={median(fast):.0f}  "
          f"slow={len(slow)} median={median(slow):.0f}  ratio={median(fast)/median(slow):.2f}x")
    print(f"runs between 100,000 and 160,000: {sum(100_000 <= t <= 160_000 for t in tp)}")

    cfg: dict[tuple[str, int], list[float]] = defaultdict(list)
    for r in rows:
        cfg[(r["strategy"], int(r["concurrency"]))].append(float(r["throughput"]))
    mixed = changes = 0
    for series in cfg.values():
        states = [t > CUT for t in series]
        if len(set(states)) > 1:
            mixed += 1
        changes += sum(a != b for a, b in zip(states, states[1:]))
    print(f"configurations={len(cfg)}  containing both states={mixed}  state changes within configs={changes}")

    print("median throughput (ops/s) at c=200:",
          {s: round(median(v)) for (s, c), v in sorted(cfg.items()) if c == 200})

    print("fast-state share by strategy:")
    by_s: dict[str, list[float]] = defaultdict(list)
    for (s, _), v in cfg.items():
        by_s[s].extend(v)
    for s, v in sorted(by_s.items()):
        n_fast = sum(t > CUT for t in v)
        print(f"  {s:<14} {n_fast:>3}/{len(v)} ({100*n_fast/len(v):.0f}%)")

    main_med = median(cfg[("sharded_2key", 500)])
    rerun = [float(r["throughput"]) for r in csv.DictReader(
        (ROOT / "anomaly_investigation" / "anomaly_summary.csv").open())]
    print(f"sharded_2key c=500: main-run median={main_med:.0f}  rerun median={median(rerun):.0f} (n={len(rerun)})")


if __name__ == "__main__":
    main()
