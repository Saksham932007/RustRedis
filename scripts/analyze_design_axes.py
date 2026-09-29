#!/usr/bin/env python3
"""Factorial decomposition of per-command observability overhead into design axes.

This script produces every number quoted in the design-axis paper
(docs/paper_design_axes.md). It re-derives them from the committed raw CSVs,
so `python3 scripts/analyze_design_axes.py` regenerates the paper's tables.

Background
----------
The six metrics strategies in src/command_metrics.rs are not an arbitrary list:
they differ along four *separable* design axes, so differences between adjacent
pairs isolate one axis each.

  strategy        synchronization        cardinality  aggregation      payload
  --------------  ---------------------  -----------  ---------------  ------------
  disabled        (none)                 0            (none)           (none)
  global_mutex    one global Mutex       2            eager, shared    counter
  sharded_2key    DashMap, 64 shards     2            eager, shared    counter
  sharded_n       DashMap, 64 shards     ~10 000      eager, shared    counter
  thread_local    thread-local + flush   2            deferred, priv.  counter
  hdr_histogram   thread-local + flush   2            deferred, priv.  counter + HDR

Contrasts (each varies exactly one axis, holding the others fixed):

  SYNC    = global_mutex  - sharded_2key   global lock -> 64-way sharded map
  CARD    = sharded_n     - sharded_2key   cardinality 2 -> ~10 000
  DEFER   = sharded_2key  - thread_local   eager shared -> deferred thread-local
  PAYLOAD = hdr_histogram - thread_local   counter -> counter + HDR histogram

Statistics
----------
Overhead is always a *paired within-block* quantity: strategy and `disabled`
are compared inside the same RMIT repetition block, which is what makes the
comparison valid under machine-state drift (see the RMIT protocol docs).

For the 11-run pooled D8s_v6 dataset the 165 blocks per cell are 11 independent
runs x 15 correlated within-run blocks, so every confidence interval here is
*cluster-aware*: per-block values are first collapsed to one mean per run, and
the interval is formed over those 11 run-level numbers with t(10). This is the
same conservative accounting used by scripts/compute_mde_clustered.py.

The single-run D4s_v6 dataset has no run-level replication, so its intervals
cluster on the 30 repetition blocks instead (bootstrap over blocks).
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from collections import defaultdict
from math import comb
from pathlib import Path
from typing import Callable, Dict, List, Sequence, Tuple

import numpy as np

REPO = Path(__file__).resolve().parent.parent
POOLED = REPO / "experiments" / "azure_d8s_v6" / "pooled" / "raw_data_rmit.csv"
D4S = REPO / "experiments" / "azure_d4s_v6" / "raw_data_rmit.csv"

STRATEGIES = ["disabled", "global_mutex", "sharded_2key", "sharded_n",
              "thread_local", "hdr_histogram"]
COSTLY = ["thread_local", "sharded_2key", "global_mutex", "hdr_histogram", "sharded_n"]

# Each contrast varies exactly one design axis.
CONTRASTS = {
    "SYNC":    ("global_mutex",  "sharded_2key"),
    "CARD":    ("sharded_n",     "sharded_2key"),
    "DEFER":   ("sharded_2key",  "thread_local"),
    "PAYLOAD": ("hdr_histogram", "thread_local"),
}

# "cost" is always oriented so that higher = worse.
METRICS = ["throughput", "avg_latency", "p50", "p99"]
T_CRIT_10 = 2.228  # two-sided 97.5th percentile of t with 10 df (11 runs)


# --------------------------------------------------------------------------- #
# loading
# --------------------------------------------------------------------------- #
def load_blocks(path: Path, workload: str | None = None,
                concurrency: Sequence[int] | None = None) -> Dict[tuple, dict]:
    """Return {(block_id, concurrency, workload): {strategy: metrics}} for
    blocks where all six strategies are present."""
    conc_ok = set(concurrency) if concurrency else None
    cells: Dict[tuple, dict] = defaultdict(dict)
    with path.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            wl = row.get("workload") or "mixed"
            c = int(row["concurrency"])
            if workload is not None and wl != workload:
                continue
            if conc_ok is not None and c not in conc_ok:
                continue
            # `sharded_bucketed` appears once per cardinality, so the config id
            # (not the bare strategy name) is what identifies a column.
            card = (row.get("metric_cardinality") or "").strip()
            config_id = row["strategy"] if not card else f"{row['strategy']}_c{card}"
            cells[(int(row["block_id"]), c, wl)][config_id] = {
                "achieved_entries": (row.get("achieved_metric_entries") or "").strip(),
                "throughput": float(row["throughput"]),
                "p50": float(row["p50"]),
                "p99": float(row["p99"]),
                "avg_latency": float(row["avg_latency"]),
                "latency_cv": float(row["latency_cv"]),
                "run": row.get("source_iteration") or "single",
            }
    return {k: v for k, v in cells.items()
            if all(s in v for s in STRATEGIES)}


def cost(block: dict, strategy: str, metric: str) -> float:
    """Percentage degradation vs the `disabled` baseline in the same block.

    Throughput is a higher-is-better metric, latencies are lower-is-better, so
    both are converted to a "cost" where a positive number means instrumentation
    made things worse.
    """
    base = block["disabled"][metric]
    value = block[strategy][metric]
    if metric == "throughput":
        return (1.0 - value / base) * 100.0
    return (value / base - 1.0) * 100.0


# --------------------------------------------------------------------------- #
# statistics
# --------------------------------------------------------------------------- #
def by_run(blocks: Dict[tuple, dict], fn: Callable[[dict], float]) -> np.ndarray:
    """Collapse per-block values to one mean per independent run (cluster)."""
    buckets: Dict[str, List[float]] = defaultdict(list)
    for block in blocks.values():
        buckets[block["disabled"]["run"]].append(fn(block))
    return np.array([float(np.mean(v)) for _, v in sorted(buckets.items())])


def cluster_ci(run_means: np.ndarray) -> Tuple[float, float, float]:
    """Mean and t-interval over independent runs (the cluster unit)."""
    n = len(run_means)
    if n < 2:
        m = float(run_means.mean())
        return m, float("nan"), float("nan")
    se = float(run_means.std(ddof=1)) / math.sqrt(n)
    m = float(run_means.mean())
    return m, m - T_CRIT_10 * se, m + T_CRIT_10 * se


def block_bootstrap_ci(blocks: Dict[tuple, dict], fn: Callable[[dict], float],
                       n_boot: int = 20000, seed: int = 0) -> Tuple[float, float, float]:
    """For the single-run dataset: bootstrap over repetition blocks.

    Values are first averaged within each block_id, so the resampling unit is
    the repetition block (the independent unit of that design) rather than the
    individual block x concurrency cell.
    """
    per_block: Dict[int, List[float]] = defaultdict(list)
    for key, block in blocks.items():
        per_block[key[0]].append(fn(block))
    vals = np.array([np.mean(v) for v in per_block.values()])
    rng = np.random.default_rng(seed)
    boots = rng.choice(vals, size=(n_boot, len(vals)), replace=True).mean(axis=1)
    return float(vals.mean()), float(np.percentile(boots, 2.5)), float(np.percentile(boots, 97.5))


def sign_test_p(n_positive: int, n: int) -> float:
    """Exact two-sided sign test."""
    k = max(n_positive, n - n_positive)
    tail = sum(comb(n, i) for i in range(k, n + 1)) / 2 ** n
    return min(1.0, 2 * tail)


def slope_per_doubling(blocks: Dict[tuple, dict],
                       fn: Callable[[dict], float]) -> Tuple[float, float, float, int, int]:
    """OLS slope of the quantity on log2(concurrency), fitted within each run,
    then averaged across runs with a cluster-aware t-interval.

    Fitting within run and aggregating across runs keeps between-run level
    differences from leaking into the slope estimate.
    """
    points: Dict[str, List[Tuple[int, float]]] = defaultdict(list)
    for key, block in blocks.items():
        points[block["disabled"]["run"]].append((key[1], fn(block)))
    slopes = []
    for _, pts in sorted(points.items()):
        x = np.log2(np.array([p[0] for p in pts], dtype=float))
        y = np.array([p[1] for p in pts], dtype=float)
        x = x - x.mean()
        slopes.append(float((x * y).sum() / (x * x).sum()))
    s = np.array(slopes)
    m, lo, hi = cluster_ci(s)
    return m, lo, hi, len(s), int((s > 0).sum())


def contrast_fn(a: str, b: str, metric: str) -> Callable[[dict], float]:
    return lambda blk: cost(blk, a, metric) - cost(blk, b, metric)


# --------------------------------------------------------------------------- #
# reporting sections
# --------------------------------------------------------------------------- #
def hr(title: str) -> None:
    print("\n" + "=" * 78)
    print(title)
    print("=" * 78)


def section_overheads(pooled, d4s, out: dict) -> None:
    hr("TABLE 1  Per-strategy overhead by budget metric (pooled D8s_v6, 11 runs)")
    print(f"{'strategy':16s}" + "".join(f"{m:>24s}" for m in METRICS))
    out["table1"] = {}
    for s in COSTLY:
        line = f"{s:16s}"
        out["table1"][s] = {}
        for m in METRICS:
            mu, lo, hi = cluster_ci(by_run(pooled, lambda b, s=s, m=m: cost(b, s, m)))
            out["table1"][s][m] = dict(mean=mu, ci_lo=lo, ci_hi=hi)
            line += f"  {mu:6.3f} [{lo:6.3f},{hi:6.3f}]"
        print(line)
    print("\n  values are % degradation vs `disabled`, paired within RMIT block;")
    print("  95% CIs cluster on the 11 independent runs (t, 10 df).")

    print("\n  Rank order (1 = cheapest) under each budget metric:")
    out["rank_order"] = {}
    for m in METRICS:
        order = sorted(COSTLY, key=lambda s: out["table1"][s][m]["mean"])
        out["rank_order"][m] = order
        print(f"    {m:12s}: " + "  <  ".join(order))


def section_axes(pooled, d4s, out: dict) -> None:
    hr("TABLE 2  Design-axis contrasts, throughput (percentage points)")
    print("  Each row varies ONE axis and holds the other three fixed.\n")
    print(f"{'axis':10s} {'contrast':34s} {'D8s_v6 (8 workers, 11 runs)':>30s}"
          f" {'D4s_v6 (4 workers)':>24s}")
    out["table2"] = {}
    d4s_full = d4s
    for axis, (a, b) in CONTRASTS.items():
        f = contrast_fn(a, b, "throughput")
        runs = by_run(pooled, f)
        mu, lo, hi = cluster_ci(runs)
        pos = int((runs > 0).sum())
        m4, l4, h4 = block_bootstrap_ci(d4s_full, f)
        out["table2"][axis] = dict(
            contrast=f"{a} - {b}",
            d8s=dict(mean=mu, ci_lo=lo, ci_hi=hi, runs_positive=pos, n_runs=len(runs),
                     sign_p=sign_test_p(pos, len(runs))),
            d4s=dict(mean=m4, ci_lo=l4, ci_hi=h4),
        )
        print(f"{axis:10s} {a + ' - ' + b:34s}  {mu:6.3f} [{lo:6.3f},{hi:6.3f}] {pos:2d}/{len(runs)}"
              f"   {m4:6.3f} [{l4:6.3f},{h4:6.3f}]")
    print("\n  D8s_v6 CIs cluster on 11 runs; D4s_v6 is a single VM so its CI")
    print("  bootstraps over that run's 30 repetition blocks.")


def section_headline(pooled, d4s, out: dict) -> None:
    hr("RESULT 1  Cardinality costs more than synchronization discipline")
    card = by_run(pooled, contrast_fn("sharded_n", "sharded_2key", "throughput"))
    sync = by_run(pooled, contrast_fn("global_mutex", "sharded_2key", "throughput"))
    diff = card - sync
    mu, lo, hi = cluster_ci(diff)
    pos = int((diff > 0).sum())
    n = len(diff)
    se = float(diff.std(ddof=1)) / math.sqrt(n)
    print(f"  CARD (cardinality 2 -> 10 000)      {card.mean():6.3f} pp")
    print(f"  SYNC (global lock -> sharded map)   {sync.mean():6.3f} pp")
    print(f"  CARD - SYNC                         {mu:+6.3f} pp  95% CI [{lo:+.3f},{hi:+.3f}]")
    print(f"  ratio CARD/SYNC                     {card.mean() / sync.mean():5.2f}x")
    print(f"  CARD > SYNC in {pos}/{n} independent runs"
          f"   sign p = {sign_test_p(pos, n):.4f}   paired t({n - 1}) = {mu / se:.2f}")

    # The practical consequence: the sharded structure at high cardinality is
    # worse than the single global lock at low cardinality.
    gm = by_run(pooled, lambda b: cost(b, "global_mutex", "throughput"))
    sn = by_run(pooled, lambda b: cost(b, "sharded_n", "throughput"))
    d = sn - gm
    mu2, lo2, hi2 = cluster_ci(d)
    pos2 = int((d > 0).sum())
    print(f"\n  Consequence -- 64-way sharded map at cardinality ~10 000 vs ONE global lock at 2:")
    print(f"    sharded_n  {sn.mean():.3f}%   global_mutex {gm.mean():.3f}%")
    print(f"    difference {mu2:+.3f} pp  95% CI [{lo2:+.3f},{hi2:+.3f}]"
          f"  sharded_n worse in {pos2}/{len(d)} runs, sign p = {sign_test_p(pos2, len(d)):.4f}")

    d4 = d4s
    c4 = block_bootstrap_ci(d4, contrast_fn("sharded_n", "sharded_2key", "throughput"))
    s4 = block_bootstrap_ci(d4, contrast_fn("global_mutex", "sharded_2key", "throughput"))
    print(f"\n  Same comparison on D4s_v6 (4 workers): CARD {c4[0]:.3f} pp vs SYNC {s4[0]:.3f} pp"
          f"  ratio {c4[0] / s4[0]:.2f}x")
    out["result1"] = dict(card=float(card.mean()), sync=float(sync.mean()),
                          diff_mean=mu, diff_ci=[lo, hi],
                          ratio=float(card.mean() / sync.mean()),
                          runs_positive=pos, n_runs=n, sign_p=sign_test_p(pos, n),
                          t=mu / se,
                          sharded_n_vs_global_mutex=dict(
                              mean=mu2, ci=[lo2, hi2], runs_positive=pos2,
                              sign_p=sign_test_p(pos2, len(d))),
                          d4s_card=c4[0], d4s_sync=s4[0], d4s_ratio=c4[0] / s4[0])


def section_scaling(pooled, d4s, out: dict) -> None:
    hr("RESULT 2  The axes obey different scaling laws")
    print("  (a) Sensitivity to CLIENT concurrency: slope in pp per doubling of clients,")
    print("      fitted within run over 100..3000 clients (a 30x range), 8 workers throughout.\n")
    print(f"{'quantity':30s} {'slope':>8s} {'95% CI':>20s} {'pos':>6s}  {'c=100':>8s} {'c=3000':>8s} {'ratio':>7s}")
    out["scaling_clients"] = {}
    conc = sorted({k[1] for k in pooled})

    def levels(fn):
        lo = float(np.mean([fn(b) for k, b in pooled.items() if k[1] == conc[0]]))
        hi = float(np.mean([fn(b) for k, b in pooled.items() if k[1] == conc[-1]]))
        return lo, hi

    rows = [(f"overhead[{s}]", lambda b, s=s: cost(b, s, "throughput")) for s in COSTLY]
    rows += [(f"axis {ax}", contrast_fn(a, b, "throughput")) for ax, (a, b) in CONTRASTS.items()]
    for name, fn in rows:
        m, lo, hi, n, pos = slope_per_doubling(pooled, fn)
        c_lo, c_hi = levels(fn)
        out["scaling_clients"][name] = dict(slope=m, ci_lo=lo, ci_hi=hi, pos=pos, n=n,
                                            at_100=c_lo, at_3000=c_hi,
                                            ratio=c_hi / c_lo if c_lo else float("nan"))
        print(f"{name:30s} {m:8.4f}  [{lo:7.4f},{hi:7.4f}] {pos:3d}/{n} {c_lo:8.3f} {c_hi:8.3f}"
              f" {c_hi / c_lo if c_lo else float('nan'):7.2f}x")

    print("\n  (b) Sensitivity to WORKER THREADS: same axes at 4 vs 8 vCPU (= tokio worker")
    print("      threads), matched on workload (mixed) and concurrency (<=1000).\n")
    shared_conc = {100, 250, 500, 750, 1000}
    d8_matched = load_blocks(POOLED, workload="mixed", concurrency=shared_conc)
    print(f"{'axis':10s} {'4 workers':>20s} {'8 workers':>20s} {'ratio':>8s} {'delta pp':>10s}")
    out["scaling_workers"] = {}
    for axis, (a, b) in CONTRASTS.items():
        f = contrast_fn(a, b, "throughput")
        m4, l4, h4 = block_bootstrap_ci(d4s, f)
        r8 = by_run(d8_matched, f)
        m8, l8, h8 = cluster_ci(r8)
        out["scaling_workers"][axis] = dict(
            w4=dict(mean=m4, ci=[l4, h4]), w8=dict(mean=m8, ci=[l8, h8]),
            ratio=m8 / m4, delta=m8 - m4)
        print(f"{axis:10s} {m4:6.3f} [{l4:6.3f},{h4:6.3f}] {m8:6.3f} [{l8:6.3f},{h8:6.3f}]"
              f" {m8 / m4:7.2f}x {m8 - m4:+9.3f}")
    print("\n  n.b. two worker-count points only; the worker-count column is a")
    print("  direction-of-effect observation, not a fitted scaling law.")

    print("\n  (c) Baseline load actually present (so the flat slopes are not an artifact")
    print("      of an unloaded server):\n")
    print(f"{'clients':>8s} {'throughput':>12s} {'p50 ms':>9s} {'p99 ms':>9s}")
    out["baseline_load"] = {}
    for c in conc:
        sel = [b["disabled"] for k, b in pooled.items() if k[1] == c]
        t = float(np.median([x["throughput"] for x in sel]))
        p50 = float(np.median([x["p50"] for x in sel])) / 1000.0
        p99 = float(np.median([x["p99"] for x in sel])) / 1000.0
        out["baseline_load"][c] = dict(throughput=t, p50_ms=p50, p99_ms=p99)
        print(f"{c:8d} {t:12.0f} {p50:9.2f} {p99:9.2f}")


def section_rigidity(pooled, d4s, out: dict) -> None:
    hr("RESULT 3  Overhead rigidity: which costs survive into the tail")
    print("  Tail-concentration index = (p99 cost) / (p50 cost), per run then averaged.")
    print("  index << 1: overhead is absorbed by queueing slack and largely absent at p99.")
    print("  index ~ 1 : overhead persists undiminished into the tail.\n")
    print(f"{'strategy':16s} {'p50 cost %':>11s} {'p99 cost %':>11s} {'index':>7s} {'95% CI':>16s}")
    out["rigidity"] = {}
    for s in COSTLY:
        p50 = by_run(pooled, lambda b, s=s: cost(b, s, "p50"))
        p99 = by_run(pooled, lambda b, s=s: cost(b, s, "p99"))
        idx = p99 / p50
        m, lo, hi = cluster_ci(idx)
        out["rigidity"][s] = dict(p50=float(p50.mean()), p99=float(p99.mean()),
                                   index=m, ci_lo=lo, ci_hi=hi)
        print(f"{s:16s} {p50.mean():11.3f} {p99.mean():11.3f} {m:7.2f} [{lo:6.2f},{hi:6.2f}]")

    print("\n  Per-axis cost measured on each budget metric (pp). A metric-invariant row")
    print("  is a RIGID cost; a row that shrinks from p50 to p99 is an ELASTIC cost.\n")
    print(f"{'axis':10s}" + "".join(f"{m:>22s}" for m in METRICS))
    out["axis_by_metric"] = {}
    for axis, (a, b) in CONTRASTS.items():
        line = f"{axis:10s}"
        out["axis_by_metric"][axis] = {}
        for m in METRICS:
            runs = by_run(pooled, contrast_fn(a, b, m))
            mu, lo, hi = cluster_ci(runs)
            out["axis_by_metric"][axis][m] = dict(
                mean=mu, ci_lo=lo, ci_hi=hi, runs_positive=int((runs > 0).sum()),
                sign_p=sign_test_p(int((runs > 0).sum()), len(runs)))
            line += f" {mu:+6.3f}[{lo:+.2f},{hi:+.2f}]"
        print(line)

    hr("RESULT 3b  The rank inversion a throughput-only study cannot see")
    out["inversions"] = {}
    for a, b in [("hdr_histogram", "global_mutex"), ("hdr_histogram", "sharded_n")]:
        print(f"\n  {a}  minus  {b}:")
        out["inversions"][f"{a}_vs_{b}"] = {}
        for m in METRICS:
            runs = by_run(pooled, contrast_fn(a, b, m))
            mu, lo, hi = cluster_ci(runs)
            pos = int((runs > 0).sum())
            p = sign_test_p(pos, len(runs))
            out["inversions"][f"{a}_vs_{b}"][m] = dict(
                mean=mu, ci_lo=lo, ci_hi=hi, runs_positive=pos, n_runs=len(runs), sign_p=p)
            verdict = ("costlier" if mu > 0 else "cheaper")
            resolved = "resolved" if (lo > 0 or hi < 0) else "UNRESOLVED"
            print(f"    {m:12s} {mu:+7.3f} pp [{lo:+.3f},{hi:+.3f}]  {pos:2d}/{len(runs)} runs"
                  f"  sign p={p:.4f}  -> {a} {verdict} ({resolved})")


def section_cv(pooled, out: dict) -> None:
    hr("SUPPLEMENTARY  Latency dispersion (coefficient of variation) vs disabled")
    print(f"{'strategy':16s} {'CV change %':>12s} {'95% CI':>18s}")
    out["cv"] = {}
    for s in COSTLY:
        runs = by_run(pooled, lambda b, s=s: (b[s]["latency_cv"] / b["disabled"]["latency_cv"] - 1) * 100)
        mu, lo, hi = cluster_ci(runs)
        out["cv"][s] = dict(mean=mu, ci_lo=lo, ci_hi=hi)
        print(f"{s:16s} {mu:+12.3f} [{lo:+.3f},{hi:+.3f}]")


# --------------------------------------------------------------------------- #
# Follow-up decomposition (only runs when the extra strategies are present)
# --------------------------------------------------------------------------- #
#: Contrasts that split the two axes the six-strategy design confounds.
#: Each varies one thing; see docs/followup_experiment_protocol.md.
FOLLOWUP_CONTRASTS = {
    "ALLOC":      ("sharded_bucketed_c1", "sharded_2key"),
    "PAYLOAD*":   ("hdr_histogram",       "thread_local_owned"),
    "ALLOC(tls)": ("thread_local_owned",  "thread_local"),
}


def section_followup(blocks: Dict[tuple, dict], out: dict) -> None:
    """Price the confounded axes separately, if the follow-up data is present."""
    present = set()
    for blk in blocks.values():
        present |= set(blk)
    needed = {"sharded_bucketed_c1", "thread_local_owned"}
    if not needed <= present:
        return

    hr("FOLLOW-UP  Splitting the confounded axes")
    single_run = len({b["disabled"]["run"] for b in blocks.values()}) < 2

    def interval(fn):
        if single_run:
            return block_bootstrap_ci(blocks, fn)
        runs = by_run(blocks, fn)
        return cluster_ci(runs)

    print(f"{'axis':12s} {'contrast':46s} {'pp':>8s} {'95% CI':>20s}")
    out["followup"] = {}
    for axis, (a, b) in FOLLOWUP_CONTRASTS.items():
        if a not in present or b not in present:
            continue
        m, lo, hi = interval(contrast_fn(a, b, "throughput"))
        out["followup"][axis] = dict(contrast=f"{a} - {b}", mean=m, ci_lo=lo, ci_hi=hi)
        print(f"{axis:12s} {a + ' - ' + b:46s} {m:8.3f}  [{lo:7.3f},{hi:7.3f}]")

    # The cardinality curve: sharded_bucketed against itself at rising C, so the
    # only thing that differs between rows is the number of live map entries.
    buckets = sorted(
        (c for c in present if c.startswith("sharded_bucketed_c")),
        key=lambda c: int(c.rsplit("_c", 1)[1]),
    )
    if len(buckets) >= 2:
        base = buckets[0]
        print(f"\n  Cardinality curve — each row is `{base}` held fixed as the reference,")
        print("  so every difference is entry count alone:\n")
        print(f"{'config':26s} {'requested C':>12s} {'achieved entries':>17s} "
              f"{'cost vs disabled':>17s} {'vs C=1':>16s}")
        out["cardinality_curve"] = {}
        for cfg in buckets:
            achieved = [blk[cfg].get("achieved_entries") for blk in blocks.values()
                        if blk[cfg].get("achieved_entries")]
            ach = f"{int(statistics_median(achieved))}" if achieved else "?"
            abs_m, abs_lo, abs_hi = interval(lambda b, cfg=cfg: cost(b, cfg, "throughput"))
            if cfg == base:
                rel = "     (reference)"
            else:
                r_m, r_lo, r_hi = interval(contrast_fn(cfg, base, "throughput"))
                rel = f"{r_m:+7.3f} pp"
            out["cardinality_curve"][cfg] = dict(achieved_entries=ach, cost=abs_m,
                                                 ci_lo=abs_lo, ci_hi=abs_hi)
            print(f"{cfg:26s} {cfg.rsplit('_c', 1)[1]:>12s} {ach:>17s} "
                  f"{abs_m:12.3f} %   {rel:>16s}")
        print("\n  achieved entries includes +2 control-plane entries (PING, CMDSTAT);")
        print("  data-plane entries are that value minus 2.")


def statistics_median(values):
    import statistics as _s
    return _s.median([float(v) for v in values])


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--json-out", default=str(REPO / "experiments" / "design_axes_analysis.json"))
    args = ap.parse_args()

    pooled = load_blocks(POOLED)
    d4s = load_blocks(D4S)
    print(f"pooled D8s_v6 complete blocks: {len(pooled)}"
          f"   (11 runs x 15 blocks x 8 concurrency x 3 workloads = {11 * 15 * 8 * 3})")
    print(f"single D4s_v6 complete blocks: {len(d4s)}"
          f"   (1 run x 30 blocks x 8 concurrency x 1 workload = {30 * 8})")

    out: Dict[str, object] = {"n_blocks_pooled": len(pooled), "n_blocks_d4s": len(d4s)}
    section_overheads(pooled, d4s, out)
    section_axes(pooled, d4s, out)
    section_headline(pooled, d4s, out)
    section_scaling(pooled, d4s, out)
    section_rigidity(pooled, d4s, out)
    section_cv(pooled, out)
    section_followup(pooled, out)

    Path(args.json_out).write_text(json.dumps(out, indent=1))
    print(f"\nwrote {args.json_out}")


if __name__ == "__main__":
    main()
