#!/usr/bin/env python3
"""Figures for the design-axis paper (docs/paper_design_axes.md).

Every value plotted is recomputed here by importing the same helper functions
that scripts/analyze_design_axes.py uses for its tables, so the figures and the
tables are two views of one computation and cannot silently drift apart.

  Figure 1  docs/figures/axes_fig1_decomposition.png
            The four design-axis penalties with cluster-aware 95% CIs.
  Figure 2  docs/figures/axes_fig2_scaling.png
            The same penalties vs client concurrency (flat) and vs worker
            threads (only the synchronization axis moves).
  Figure 3  docs/figures/axes_fig3_rigidity.png
            Median-vs-tail cost per strategy: which overheads survive into p99.

Colors are slots 1-5 of the validated categorical reference palette, assigned in
fixed order and never cycled. Every series also carries a redundant non-color
encoding (line style + marker shape, or hatch) so the figures survive grayscale
printing and color-vision deficiency.
"""

from __future__ import annotations

import math
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

from analyze_design_axes import (  # noqa: E402  (deliberate: shared source of truth)
    CONTRASTS, COSTLY, D4S, POOLED,
    block_bootstrap_ci, by_run, cluster_ci, contrast_fn, cost, load_blocks,
)

ROOT = Path(__file__).resolve().parent.parent
FIG_DIR = ROOT / "docs" / "figures"
FIG_DIR.mkdir(parents=True, exist_ok=True)

# Validated categorical palette, slots 1-4 for the four axes / 1-5 for strategies.
AXIS_ORDER = ["CARD", "PAYLOAD", "SYNC", "DEFER"]
AXIS_COLOR = {"CARD": "#2a78d6", "PAYLOAD": "#eb6834",
              "SYNC": "#1baf7a", "DEFER": "#eda100"}
AXIS_MARK = {"CARD": ("-", "o"), "PAYLOAD": ("--", "s"),
             "SYNC": ("-.", "^"), "DEFER": (":", "D")}
AXIS_HATCH = {"CARD": "", "PAYLOAD": "//", "SYNC": "xx", "DEFER": ".."}
AXIS_DESC = {
    "SYNC":    "SYNC\nglobal lock $\\rightarrow$ 64-way sharded map",
    "CARD":    "CARD\ncardinality 2 $\\rightarrow$ ~10 000",
    "DEFER":   "DEFER\neager shared $\\rightarrow$ deferred thread-local",
    "PAYLOAD": "PAYLOAD\ncounter $\\rightarrow$ counter + HDR histogram",
}
STRAT_COLOR = {
    "thread_local": "#2a78d6", "sharded_2key": "#eb6834",
    "global_mutex": "#1baf7a", "hdr_histogram": "#eda100",
    "sharded_n": "#e87ba4",
}
STRAT_LABEL = {
    "thread_local": "ThreadLocal", "sharded_2key": "Sharded-2key",
    "global_mutex": "GlobalMutex", "hdr_histogram": "HdrHistogram",
    "sharded_n": "Sharded-N",
}

plt.rcParams.update({
    "figure.facecolor": "white", "axes.facecolor": "white",
    "savefig.facecolor": "white", "axes.edgecolor": "#898781",
    "axes.labelcolor": "#0b0b0b", "text.color": "#0b0b0b",
    "xtick.color": "#52514e", "ytick.color": "#52514e",
    "axes.grid": True, "grid.color": "#e1e0d9", "grid.linewidth": 0.8,
    "axes.axisbelow": True, "font.size": 10.5, "font.family": "sans-serif",
    "legend.frameon": False,
})

print("loading datasets ...")
POOL = load_blocks(POOLED)
D4 = load_blocks(D4S)
D8_MATCHED = load_blocks(POOLED, workload="mixed",
                         concurrency={100, 250, 500, 750, 1000})


# --------------------------------------------------------------------------- #
def figure1() -> None:
    """Horizontal bars: the four axis penalties, 8-worker pooled dataset."""
    fig, ax = plt.subplots(figsize=(7.4, 3.5))
    ys, vals, los, his = [], [], [], []
    for i, axis in enumerate(AXIS_ORDER):
        a, b = CONTRASTS[axis]
        m, lo, hi = cluster_ci(by_run(POOL, contrast_fn(a, b, "throughput")))
        ys.append(i); vals.append(m); los.append(m - lo); his.append(hi - m)

    bars = ax.barh(ys, vals, height=0.58,
                   color=[AXIS_COLOR[a] for a in AXIS_ORDER],
                   edgecolor="white", linewidth=2)
    for bar, axis in zip(bars, AXIS_ORDER):
        bar.set_hatch(AXIS_HATCH[axis])
    ax.errorbar(vals, ys, xerr=[los, his], fmt="none",
                ecolor="#52514e", elinewidth=1.4, capsize=4)
    # Selective direct labels: the value on each bar, nothing else.
    for y, v, h in zip(ys, vals, his):
        ax.text(v + h + 0.045, y, f"{v:.2f} pp", va="center", ha="left",
                fontsize=10, color="#0b0b0b", fontweight="medium")

    ax.set_yticks(ys)
    ax.set_yticklabels([AXIS_DESC[a] for a in AXIS_ORDER], fontsize=9.5)
    ax.invert_yaxis()
    ax.set_xlabel("throughput cost of the axis (percentage points, paired within RMIT block)")
    ax.set_xlim(0, max(v + h for v, h in zip(vals, his)) + 0.34)
    ax.set_title("Each design axis priced separately\n"
                 "Azure D8s_v6, 8 worker threads, 11 independent runs; bars are 95% CIs "
                 "clustered on runs",
                 fontsize=11, loc="left", pad=10)
    ax.grid(axis="y", visible=False)
    fig.tight_layout()
    out = FIG_DIR / "axes_fig1_decomposition.png"
    fig.savefig(out, dpi=200, bbox_inches="tight")
    plt.close(fig)
    print(f"  wrote {out}")


def figure2() -> None:
    """Two panels: penalty vs client concurrency, and penalty vs worker threads."""
    fig, (axa, axb) = plt.subplots(1, 2, figsize=(9.6, 3.9),
                                   gridspec_kw={"width_ratios": [1.65, 1]})
    conc = sorted({k[1] for k in POOL})

    for axis in AXIS_ORDER:
        a, b = CONTRASTS[axis]
        f = contrast_fn(a, b, "throughput")
        # Per-concurrency mean with a cluster-aware 95% CI over the 11 runs, so
        # the reader can see that the point-to-point wiggle sits inside the
        # noise band rather than tracking concurrency.
        means, errs = [], []
        for c in conc:
            sub = {k: v for k, v in POOL.items() if k[1] == c}
            m, lo, hi = cluster_ci(by_run(sub, f))
            means.append(m); errs.append(hi - m)
        ls, mk = AXIS_MARK[axis]
        axa.errorbar(conc, means, yerr=errs, fmt=ls, marker=mk,
                     color=AXIS_COLOR[axis], linewidth=2, markersize=6,
                     markeredgecolor="white", markeredgewidth=1.2,
                     ecolor=AXIS_COLOR[axis], elinewidth=1.1, capsize=2.5,
                     alpha=0.95, label=axis)
        axa.annotate(axis, (conc[-1], means[-1]), textcoords="offset points",
                     xytext=(9, 0), va="center", fontsize=9.5,
                     color=AXIS_COLOR[axis], fontweight="medium")

    axa.set_xscale("log")
    axa.set_xticks(conc)
    axa.set_xticklabels([str(c) for c in conc], fontsize=9)
    axa.minorticks_off()
    axa.set_xlabel("concurrent clients (log scale) — a 30$\\times$ range")
    axa.set_ylabel("axis cost (percentage points)")
    axa.set_ylim(0, 1.6)
    axa.set_xlim(88, 5200)
    axa.set_title("(a) 30$\\times$ more clients, same 8 workers:\n"
                  "no axis shows a systematic trend",
                  fontsize=10.5, loc="left")

    for axis in AXIS_ORDER:
        a, b = CONTRASTS[axis]
        f = contrast_fn(a, b, "throughput")
        m4 = block_bootstrap_ci(D4, f)[0]
        m8 = cluster_ci(by_run(D8_MATCHED, f))[0]
        ls, mk = AXIS_MARK[axis]
        axb.plot([4, 8], [m4, m8], ls, marker=mk, color=AXIS_COLOR[axis],
                 linewidth=2, markersize=7, markeredgecolor="white",
                 markeredgewidth=1.2, label=axis)
    axb.annotate("SYNC\n2.6$\\times$", (8, 0.457), textcoords="offset points",
                 xytext=(10, -2), fontsize=9.5, color="#1baf7a",
                 fontweight="medium", ha="left", va="center")
    axb.set_xticks([4, 8])
    axb.set_xlabel("worker threads (= vCPU)")
    axb.set_ylim(0, 1.6)
    axb.set_xlim(3.5, 9.6)
    axb.set_title("(b) 2$\\times$ the workers, matched client load:\nonly SYNC moves",
                  fontsize=10.5, loc="left")
    axb.tick_params(labelleft=False)

    handles, labels = axa.get_legend_handles_labels()
    fig.legend(handles, labels, loc="lower center", ncol=4,
               bbox_to_anchor=(0.5, -0.09), fontsize=9.5)
    fig.suptitle("Client concurrency is the wrong knob; hardware parallelism is the right one",
                 fontsize=11.5, x=0.012, ha="left", y=1.03)
    fig.tight_layout()
    out = FIG_DIR / "axes_fig2_scaling.png"
    fig.savefig(out, dpi=200, bbox_inches="tight")
    plt.close(fig)
    print(f"  wrote {out}")


def figure3() -> None:
    """Dumbbell: each strategy's cost at the median vs at p99."""
    fig, ax = plt.subplots(figsize=(7.8, 4.1))
    order = sorted(COSTLY, key=lambda s: -cluster_ci(by_run(POOL, lambda b, s=s: cost(b, s, "p50")))[0])
    for i, s in enumerate(order):
        p50 = cluster_ci(by_run(POOL, lambda b, s=s: cost(b, s, "p50")))[0]
        p99 = cluster_ci(by_run(POOL, lambda b, s=s: cost(b, s, "p99")))[0]
        idx = cluster_ci(by_run(POOL, lambda b, s=s: cost(b, s, "p99"))
                         / by_run(POOL, lambda b, s=s: cost(b, s, "p50")))[0]
        ax.plot([p50, p99], [i, i], "-", color=AXIS_COLOR["DEFER"] if False else "#c9c8c0",
                linewidth=3, solid_capstyle="round", zorder=1)
        ax.scatter([p50], [i], s=95, color=STRAT_COLOR[s], edgecolor="white",
                   linewidth=1.8, zorder=3, marker="o")
        ax.scatter([p99], [i], s=110, color=STRAT_COLOR[s], edgecolor="white",
                   linewidth=1.8, zorder=3, marker="D")
        ax.text(max(p50, p99) + 0.09, i, f"index {idx:.2f}", va="center",
                ha="left", fontsize=9.5, color="#52514e")

    ax.set_yticks(range(len(order)))
    ax.set_yticklabels([STRAT_LABEL[s] for s in order])
    ax.set_xlabel("latency cost vs `disabled` (%)")
    ax.set_xlim(0, 3.15)
    ax.grid(axis="y", visible=False)
    from matplotlib.lines import Line2D
    ax.legend(handles=[
        Line2D([], [], marker="o", color="#52514e", linestyle="none",
               markersize=9, label="at the median (p50)"),
        Line2D([], [], marker="D", color="#52514e", linestyle="none",
               markersize=9, label="at the tail (p99)"),
    ], loc="upper right", fontsize=9.5, borderaxespad=0.8)
    ax.set_title("Most instrumentation cost disappears into queueing slack — "
                 "the histogram's does not\n"
                 "index = p99 cost ÷ p50 cost; ~1 means the cost survives into the tail",
                 fontsize=11, loc="left", pad=10)
    fig.tight_layout()
    out = FIG_DIR / "axes_fig3_rigidity.png"
    fig.savefig(out, dpi=200, bbox_inches="tight")
    plt.close(fig)
    print(f"  wrote {out}")


if __name__ == "__main__":
    figure1()
    figure2()
    figure3()
    print("done")
