#!/usr/bin/env python3
"""Analyze RMIT raw_data_rmit.csv correctly.

Rules this script follows (per the corrected experiment design):
- Report medians with bootstrap confidence intervals, not just a single
  mean +/- stddev number (means are pulled around by outliers exactly in
  the runs where a "slow state" run lands).
- If a configuration's throughput distribution looks bimodal (fast/slow
  states), report the two states separately instead of collapsing them
  into one misleading average.
- Compare strategies with paired, within-block ratios: strategy A's run
  in block N is compared to strategy B's run in the *same* block N (same
  repetition, so same rough time-of-day/thermal conditions), rather than
  comparing overall means across different blocks.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Sequence, Tuple

import random


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Analyze RMIT experiment results")
    p.add_argument("--input", required=True, help="Path to raw_data_rmit.csv")
    p.add_argument("--output-dir", default=None, help="Defaults to the input file's directory")
    p.add_argument("--bootstrap-samples", type=int, default=2000)
    p.add_argument("--bimodal-gap-ratio", type=float, default=1.8,
                   help="Flag a config as two-state if the gap between the two k=2 cluster "
                        "medians exceeds this ratio")
    p.add_argument("--seed", type=int, default=0)
    return p.parse_args()


def load_rows(path: Path) -> List[Dict[str, str]]:
    with path.open("r", newline="", encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def bootstrap_median_ci(values: Sequence[float], n_samples: int, rng: random.Random) -> Tuple[float, float, float]:
    if not values:
        return 0.0, 0.0, 0.0
    med = statistics.median(values)
    if len(values) < 2:
        return med, med, med
    n = len(values)
    boot_medians = []
    for _ in range(n_samples):
        sample = [values[rng.randrange(n)] for _ in range(n)]
        boot_medians.append(statistics.median(sample))
    boot_medians.sort()
    lo_idx = int(0.025 * n_samples)
    hi_idx = int(0.975 * n_samples) - 1
    return med, boot_medians[max(0, lo_idx)], boot_medians[min(n_samples - 1, hi_idx)]


def split_two_states_1d(values: Sequence[float]) -> Tuple[List[float], List[float]]:
    """Simple 1D k=2 split: sort, find the largest single gap, cut there.

    This is a deliberately simple stand-in for a full Gaussian-mixture
    fit — good enough to separate an obvious bimodal fast/slow pattern
    without adding a numpy/scipy dependency to a benchmark script.
    """
    if len(values) < 4:
        return list(values), []
    ordered = sorted(values)
    gaps = [(ordered[i + 1] - ordered[i], i) for i in range(len(ordered) - 1)]
    gaps.sort(reverse=True)
    biggest_gap, split_idx = gaps[0]
    low = ordered[: split_idx + 1]
    high = ordered[split_idx + 1 :]
    return low, high


def detect_two_states(values: Sequence[float], gap_ratio_threshold: float) -> Dict[str, object]:
    if len(values) < 6:
        return {"bimodal": False, "reason": "too few samples to assess"}

    low, high = split_two_states_1d(values)
    if not low or not high:
        return {"bimodal": False}

    med_low = statistics.median(low)
    med_high = statistics.median(high)
    if med_low <= 0:
        return {"bimodal": False}

    ratio = med_high / med_low
    # Require both clusters to hold a meaningful share of the samples —
    # otherwise a single genuine outlier could masquerade as a "state".
    min_cluster_frac = 0.15
    frac_low = len(low) / len(values)
    frac_high = len(high) / len(values)

    is_bimodal = (
        ratio >= gap_ratio_threshold
        and frac_low >= min_cluster_frac
        and frac_high >= min_cluster_frac
    )

    return {
        "bimodal": is_bimodal,
        "ratio_high_over_low": ratio,
        "state_low_n": len(low),
        "state_low_median": med_low,
        "state_high_n": len(high),
        "state_high_median": med_high,
    }


def summarize_config(rows: List[Dict[str, str]], rng: random.Random, args: argparse.Namespace) -> Dict[str, object]:
    throughput = [float(r["throughput"]) for r in rows]
    p99 = [float(r["p99"]) for r in rows]

    t_median, t_lo, t_hi = bootstrap_median_ci(throughput, args.bootstrap_samples, rng)
    p_median, p_lo, p_hi = bootstrap_median_ci(p99, args.bootstrap_samples, rng)

    state_info = detect_two_states(throughput, args.bimodal_gap_ratio)

    return {
        "n": len(rows),
        "throughput_median": t_median,
        "throughput_ci_low": t_lo,
        "throughput_ci_high": t_hi,
        "p99_median": p_median,
        "p99_ci_low": p_lo,
        "p99_ci_high": p_hi,
        "throughput_min": min(throughput),
        "throughput_max": max(throughput),
        "two_state": state_info,
    }


def paired_ratio_comparisons(
    rows: List[Dict[str, str]],
    baseline_strategy: str,
) -> Dict[Tuple[str, int, str], Dict[str, object]]:
    """For every (strategy, concurrency, workload), compute the ratio of that
    strategy's throughput to `baseline_strategy`'s throughput within the SAME
    block — the pairing that controls for whatever time-varying machine
    state RMIT is designed to average out."""
    by_block_conc_wl: Dict[Tuple[int, int, str], Dict[str, float]] = defaultdict(dict)
    for r in rows:
        block = int(r["block_id"])
        conc = int(r["concurrency"])
        wl = r.get("workload", "mixed")
        by_block_conc_wl[(block, conc, wl)][r["strategy"]] = float(r["throughput"])

    ratios: Dict[Tuple[str, int, str], List[float]] = defaultdict(list)
    for (block, conc, wl), strategy_map in by_block_conc_wl.items():
        baseline_val = strategy_map.get(baseline_strategy)
        if baseline_val is None or baseline_val <= 0:
            continue
        for strategy, val in strategy_map.items():
            if strategy == baseline_strategy:
                continue
            ratios[(strategy, conc, wl)].append(val / baseline_val)

    result: Dict[Tuple[str, int, str], Dict[str, object]] = {}
    for key, values in ratios.items():
        result[key] = {
            "n_paired_blocks": len(values),
            "ratio_median": statistics.median(values) if values else None,
            "ratio_mean": statistics.mean(values) if values else None,
        }
    return result


def detect_ranking_crossovers(
    per_config_summary: List[Dict[str, object]],
) -> Dict[str, object]:
    """For each workload, walk concurrency levels in order and track which
    strategy has the highest median throughput. A crossover is any point
    where the leader changes — this is the signal that would justify
    picking a different strategy at different concurrency levels, which a
    single flat "overhead is X%" number would hide."""
    by_workload: Dict[str, Dict[int, List[Tuple[str, float]]]] = defaultdict(lambda: defaultdict(list))
    for s in per_config_summary:
        wl = s.get("workload", "mixed")
        by_workload[wl][s["concurrency"]].append((s["strategy"], s["throughput_median"]))

    report: Dict[str, object] = {}
    for wl, by_conc in by_workload.items():
        leaders: List[Tuple[int, str, float]] = []
        for conc in sorted(by_conc):
            strategy, median = max(by_conc[conc], key=lambda t: t[1])
            leaders.append((conc, strategy, median))

        crossovers = []
        for i in range(1, len(leaders)):
            prev_conc, prev_leader, _ = leaders[i - 1]
            conc, leader, _ = leaders[i]
            if leader != prev_leader:
                crossovers.append({
                    "from_concurrency": prev_conc,
                    "to_concurrency": conc,
                    "leader_before": prev_leader,
                    "leader_after": leader,
                })

        report[wl] = {
            "leader_by_concurrency": [
                {"concurrency": c, "leading_strategy": s, "throughput_median": m} for c, s, m in leaders
            ],
            "crossovers": crossovers,
        }
    return report


def main() -> None:
    args = parse_args()
    input_path = Path(args.input).resolve()
    output_dir = Path(args.output_dir).resolve() if args.output_dir else input_path.parent

    rows = load_rows(input_path)
    if not rows:
        raise SystemExit(f"No rows found in {input_path}")

    rng = random.Random(args.seed)

    grouped: Dict[Tuple[str, int, str], List[Dict[str, str]]] = defaultdict(list)
    for r in rows:
        grouped[(r["strategy"], int(r["concurrency"]), r.get("workload", "mixed"))].append(r)

    per_config_summary = []
    for (strategy, concurrency, workload), cfg_rows in sorted(grouped.items()):
        summary = summarize_config(cfg_rows, rng, args)
        summary["strategy"] = strategy
        summary["concurrency"] = concurrency
        summary["workload"] = workload
        per_config_summary.append(summary)

    two_state_configs = [s for s in per_config_summary if s["two_state"].get("bimodal")]

    baseline = "disabled"
    if not any(r["strategy"] == baseline for r in rows):
        baseline = sorted({r["strategy"] for r in rows})[0]
    ratios = paired_ratio_comparisons(rows, baseline_strategy=baseline)

    crossovers = detect_ranking_crossovers(per_config_summary)
    total_crossovers = sum(len(v["crossovers"]) for v in crossovers.values())

    report = {
        "input_file": str(input_path),
        "n_raw_rows": len(rows),
        "n_configs": len(per_config_summary),
        "n_configs_flagged_two_state": len(two_state_configs),
        "two_state_config_keys": [
            {"strategy": s["strategy"], "concurrency": s["concurrency"], "workload": s["workload"], **s["two_state"]}
            for s in two_state_configs
        ],
        "per_config_summary": per_config_summary,
        "paired_ratio_baseline": baseline,
        "paired_ratio_vs_baseline": [
            {"strategy": k[0], "concurrency": k[1], "workload": k[2], **v} for k, v in sorted(ratios.items())
        ],
        "ranking_crossovers_by_workload": crossovers,
    }

    out_json = output_dir / "rmit_analysis.json"
    out_json.write_text(json.dumps(report, indent=2), encoding="utf-8")

    out_csv = output_dir / "rmit_analysis_summary.csv"
    with out_csv.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(
            fh,
            fieldnames=[
                "strategy", "concurrency", "workload", "n", "throughput_median", "throughput_ci_low",
                "throughput_ci_high", "p99_median", "p99_ci_low", "p99_ci_high", "two_state_bimodal",
            ],
        )
        writer.writeheader()
        for s in per_config_summary:
            writer.writerow({
                "strategy": s["strategy"],
                "concurrency": s["concurrency"],
                "workload": s["workload"],
                "n": s["n"],
                "throughput_median": s["throughput_median"],
                "throughput_ci_low": s["throughput_ci_low"],
                "throughput_ci_high": s["throughput_ci_high"],
                "p99_median": s["p99_median"],
                "p99_ci_low": s["p99_ci_low"],
                "p99_ci_high": s["p99_ci_high"],
                "two_state_bimodal": s["two_state"].get("bimodal", False),
            })

    print(f"Configs analyzed: {len(per_config_summary)}")
    print(f"Configs flagged as two-state (fast/slow): {len(two_state_configs)}")
    for s in two_state_configs:
        ts = s["two_state"]
        print(
            f"  {s['strategy']}/c{s['concurrency']}/{s['workload']}: "
            f"low={ts['state_low_median']:.0f} ops/s (n={ts['state_low_n']}) vs "
            f"high={ts['state_high_median']:.0f} ops/s (n={ts['state_high_n']}), "
            f"ratio={ts['ratio_high_over_low']:.2f}x"
        )
    print(f"\nRanking crossovers (strategy with highest median throughput changes "
          f"as concurrency increases): {total_crossovers} total")
    for wl, info in crossovers.items():
        for c in info["crossovers"]:
            print(f"  [{wl}] c={c['from_concurrency']}->{c['to_concurrency']}: "
                  f"{c['leader_before']} -> {c['leader_after']}")
    print(f"\nWritten: {out_json}")
    print(f"Written: {out_csv}")


if __name__ == "__main__":
    main()
