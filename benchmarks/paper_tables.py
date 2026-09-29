#!/usr/bin/env python3
"""Regenerate every numeric claim in paper §4/§4b from the three rmit_analysis.json files.

Overhead = 1 - median paired-block throughput ratio (strategy / disabled) per
(strategy, concurrency[, workload]) cell; "mean overhead" = arithmetic mean of
those per-cell values over the cells named. Run from the repo root.
"""

from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path
from statistics import mean

ROOT = Path(__file__).resolve().parent.parent
DATASETS = [
    ("azure_d4s_v6", "experiment_results_rmit_azure"),
    ("azure_d8s_v6_advanced_1x", "experiment_results_rmit_advanced"),
    ("azure_d8s_v6_advanced_11x_pooled", "experiment_results_adv_pooled"),
]
STRATS = ["thread_local", "sharded_2key", "global_mutex", "hdr_histogram", "sharded_n"]


def load(d: str) -> dict:
    return json.load(open(ROOT / d / "rmit_analysis.json"))


def oh(r: dict) -> float:
    return (1.0 - r["ratio_median"]) * 100.0


def main() -> None:
    for name, d in DATASETS:
        a = load(d)
        pr = a["paired_ratio_vs_baseline"]
        cfg = a["per_config_summary"]
        print(f"\n=== {name} ===")
        print(f"configs={a['n_configs']} flagged_two_state={a['n_configs_flagged_two_state']}")
        widths = [(c["throughput_ci_high"] - c["throughput_ci_low"]) / c["throughput_median"] for c in cfg]
        print(f"relative CI width: mean={mean(widths):.4f} max={max(widths):.4f}")
        by_s = defaultdict(list)
        for r in pr:
            by_s[r["strategy"]].append(oh(r))
        means = {s: mean(by_s[s]) for s in STRATS}
        print("mean overhead by strategy (%):", {s: round(v, 2) for s, v in means.items()})
        print("rank order (cheapest first):", sorted(STRATS, key=means.get))

        by_sc = defaultdict(list)
        for r in pr:
            by_sc[(r["strategy"], r["concurrency"])].append(oh(r))
        conc_avgs = {k: mean(v) for k, v in by_sc.items()}
        print(f"max per-concurrency average overhead (over workloads) = {max(conc_avgs.values()):.2f}% "
              f"at {max(conc_avgs, key=conc_avgs.get)}")
        print(f"max single-cell overhead = {max(oh(r) for r in pr):.2f}%  "
              f"min single-cell overhead = {min(oh(r) for r in pr):.2f}%")
        print(f"per-cell ratio_median range: {min(r['ratio_median'] for r in pr):.4f} .. "
              f"{max(r['ratio_median'] for r in pr):.4f}; ratio_mean range: "
              f"{min(r['ratio_mean'] for r in pr):.4f} .. {max(r['ratio_mean'] for r in pr):.4f}")

        if "workload" in pr[0]:
            print("overhead by workload (mean over concurrency, %):")
            for s in STRATS:
                row = {w: round(mean(oh(r) for r in pr if r["strategy"] == s and r["workload"] == w), 2)
                       for w in ("mixed", "read-heavy", "write-heavy")}
                print(f"  {s:<14}{row}")
            print("per-concurrency table (avg over workloads, %):")
            for c in sorted({k[1] for k in conc_avgs}):
                print(f"  {c:>5}", [round(conc_avgs[(s, c)], 2) for s in
                                    ["global_mutex", "hdr_histogram", "sharded_2key", "sharded_n", "thread_local"]])
            n_cross = sum(
                sum(1 for x, y in zip(v["leader_by_concurrency"], v["leader_by_concurrency"][1:])
                    if x["leading_strategy"] != y["leading_strategy"])
                for v in a["ranking_crossovers_by_workload"].values()
            )
            print(f"leader changes across workloads (adjacent concurrency levels) = {n_cross}")
            base = {(c["workload"], c["concurrency"]): c for c in cfg if c["strategy"] == "disabled"}
            print("disabled baseline throughput (k ops/s) / p99 (ms), by workload:")
            for w in ("mixed", "read-heavy", "write-heavy"):
                for c in (100, 250, 3000):
                    b = base[(w, c)]
                    print(f"  {w:<12}c={c:<5} {b['throughput_median']/1000:6.1f}k  p99={b['p99_median']/1000:6.2f}ms")

        rows = list(csv.DictReader(open(ROOT / d / "raw_data_rmit.csv")))
        loads = [float(r["load_1m"]) for r in rows if r.get("load_1m") not in (None, "")]
        if loads:
            print(f"load_1m: min={min(loads):.1f} median={sorted(loads)[len(loads)//2]:.1f} max={max(loads):.1f}")


if __name__ == "__main__":
    main()
