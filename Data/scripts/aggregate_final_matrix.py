#!/usr/bin/env python3
import csv
import json
from pathlib import Path

import matplotlib.pyplot as plt

BASE = Path("results/final_matrix/20260408_154844")
CORES = ["core_4", "core_8"]
STRATEGIES = ["disabled", "global_mutex", "sharded", "thread_local"]
CONCURRENCY = [100, 500, 1000]


def load_rows():
    rows = []
    for core in CORES:
        for strategy in STRATEGIES:
            path = BASE / core / strategy / "benchmark_results.json"
            data = json.loads(path.read_text())
            for result in sorted(data["results"], key=lambda x: x["concurrency"]):
                if not result["name"].lower().startswith("mixed"):
                    continue
                t_mean = result["ops_per_sec_mean"]
                t_std = result["ops_per_sec_stddev"]
                p99_mean = result["p99_us_mean"]
                p99_std = result["p99_us_stddev"]
                rows.append(
                    {
                        "core_setup": core.replace("core_", "") + "-core",
                        "strategy": strategy,
                        "clients": result["concurrency"],
                        "throughput_mean": t_mean,
                        "throughput_stddev": t_std,
                        "throughput_cv": (t_std / t_mean) if t_mean else 0.0,
                        "p99_mean_us": p99_mean,
                        "p99_stddev_us": p99_std,
                        "p99_cv": (p99_std / p99_mean) if p99_mean else 0.0,
                        "total_errors": result.get("total_errors", 0),
                    }
                )
    return rows


def add_overheads(rows):
    idx = {(r["core_setup"], r["strategy"], r["clients"]): r for r in rows}
    for r in rows:
        baseline = idx[(r["core_setup"], "disabled", r["clients"])]
        r["throughput_overhead_pct_vs_disabled"] = (
            1 - (r["throughput_mean"] / baseline["throughput_mean"])
        ) * 100.0
        r["p99_increase_pct_vs_disabled"] = (
            (r["p99_mean_us"] / baseline["p99_mean_us"]) - 1
        ) * 100.0


def write_csv(rows):
    out = BASE / "observability_matrix_summary.csv"
    with out.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)
    return out


def write_markdown(rows):
    idx = {(r["core_setup"], r["strategy"], r["clients"]): r for r in rows}
    out = BASE / "observability_matrix_summary.md"
    with out.open("w") as f:
        f.write(
            "| Core | Strategy | Clients | Throughput mean +- stddev | p99 mean +- stddev (us) | Throughput CV | p99 CV | Overhead vs Disabled | p99 delta vs Disabled | Errors |\n"
        )
        f.write("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n")
        for core in ["4-core", "8-core"]:
            for clients in CONCURRENCY:
                for strategy in STRATEGIES:
                    r = idx[(core, strategy, clients)]
                    f.write(
                        f"| {core} | {strategy} | {clients} | {r['throughput_mean']:.2f} +- {r['throughput_stddev']:.2f} | {r['p99_mean_us']:.2f} +- {r['p99_stddev_us']:.2f} | {r['throughput_cv']:.3f} | {r['p99_cv']:.3f} | {r['throughput_overhead_pct_vs_disabled']:+.2f}% | {r['p99_increase_pct_vs_disabled']:+.2f}% | {r['total_errors']} |\n"
                    )
    return out


def write_plot(rows):
    idx = {(r["core_setup"], r["strategy"], r["clients"]): r for r in rows}
    colors = {
        "disabled": "#2c7fb8",
        "global_mutex": "#d7191c",
        "sharded": "#fdae61",
        "thread_local": "#1a9641",
    }
    labels = {
        "disabled": "Disabled",
        "global_mutex": "Global",
        "sharded": "Sharded",
        "thread_local": "ThreadLocal",
    }

    fig, axes = plt.subplots(2, 2, figsize=(12, 8), sharex="col")
    for i, core in enumerate(["4-core", "8-core"]):
        ax_t = axes[i, 0]
        ax_p = axes[i, 1]
        for strategy in STRATEGIES:
            points = [idx[(core, strategy, c)] for c in CONCURRENCY]
            ax_t.plot(
                CONCURRENCY,
                [p["throughput_mean"] for p in points],
                marker="o",
                color=colors[strategy],
                label=labels[strategy],
            )
            ax_p.plot(
                CONCURRENCY,
                [p["p99_mean_us"] for p in points],
                marker="o",
                color=colors[strategy],
                label=labels[strategy],
            )
        ax_t.set_title(f"{core} Throughput")
        ax_p.set_title(f"{core} p99 Latency")
        ax_t.set_ylabel("ops/sec")
        ax_p.set_ylabel("us")
        ax_t.set_xticks(CONCURRENCY)
        ax_p.set_xticks(CONCURRENCY)
        ax_t.grid(alpha=0.3)
        ax_p.grid(alpha=0.3)

    axes[1, 0].set_xlabel("Clients")
    axes[1, 1].set_xlabel("Clients")
    handles, lbls = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, lbls, loc="upper center", ncol=4)
    fig.suptitle("Observability Cost vs Concurrency", y=0.98)
    fig.tight_layout(rect=[0, 0, 1, 0.94])

    out = BASE / "observability_cost_vs_concurrency.png"
    fig.savefig(out, dpi=180)
    return out


def main():
    rows = load_rows()
    add_overheads(rows)
    csv_path = write_csv(rows)
    md_path = write_markdown(rows)
    fig_path = write_plot(rows)
    print(csv_path)
    print(md_path)
    print(fig_path)


if __name__ == "__main__":
    main()
