#!/usr/bin/env python3
"""Generate final v12 experiment dataset and summary from raw artifacts."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Tuple

from scipy import stats

STRATEGY_ORDER = [
    "disabled",
    "global_mutex",
    "sharded_2key",
    "thread_local",
    "hdr_histogram",
    "sharded_n",
]

STRATEGY_LABELS = {
    "disabled": "Disabled",
    "global_mutex": "GlobalMutex",
    "sharded_2key": "Sharded-2key",
    "thread_local": "ThreadLocal",
    "hdr_histogram": "HdrHistogram",
    "sharded_n": "Sharded-N",
}

CONCURRENCY_LEVELS = [100, 200, 300, 400, 500, 600, 700, 1000]
SHAPIRO_CONCURRENCY_LEVELS = [400, 500, 600, 700]
TEST_CONCURRENCY_LEVELS = [500, 600, 700]
LOW_CONCURRENCY_LEVELS = [100, 200, 300]

COMPARISONS = [
    ("disabled", "sharded_2key", "Disabled vs Sharded-2key"),
    ("disabled", "hdr_histogram", "Disabled vs HdrHistogram"),
    ("sharded_2key", "global_mutex", "Sharded-2key vs GlobalMutex"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate final v12 dataset JSON and markdown summary")
    parser.add_argument(
        "--root",
        default=".",
        help="Repository root directory",
    )
    parser.add_argument(
        "--raw-data",
        default="experiment_results_v12/raw_data.csv",
        help="Raw CSV path (relative to repo root)",
    )
    parser.add_argument(
        "--metadata",
        default="experiment_results_v12/metadata.json",
        help="Metadata JSON path (relative to repo root)",
    )
    parser.add_argument(
        "--validation-latest",
        default="experiment_results_v12/system_validation/latest_run.txt",
        help="Path containing latest system validation run directory",
    )
    parser.add_argument(
        "--output-json",
        default="final_experiment_v12.json",
        help="Output JSON path (relative to repo root)",
    )
    parser.add_argument(
        "--output-md",
        default="final_experiment_summary.md",
        help="Output markdown summary path (relative to repo root)",
    )
    parser.add_argument(
        "--config-output",
        default="experiment_results_v12/final_experiment_config.json",
        help="Output config file path (relative to repo root)",
    )
    return parser.parse_args()


def mean(values: List[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def stddev(values: List[float]) -> float:
    return statistics.stdev(values) if len(values) > 1 else 0.0


def ci95(values: List[float]) -> Tuple[float, float]:
    if not values:
        return 0.0, 0.0
    m = mean(values)
    sd = stddev(values)
    margin = 1.96 * sd / math.sqrt(len(values))
    return m - margin, m + margin


def cv(values: List[float]) -> float:
    m = mean(values)
    if m == 0:
        return 0.0
    return stddev(values) / m


def cohen_d(a: List[float], b: List[float]) -> float:
    n1 = len(a)
    n2 = len(b)
    if n1 < 2 or n2 < 2:
        return 0.0
    m1 = mean(a)
    m2 = mean(b)
    s1 = stddev(a)
    s2 = stddev(b)
    pooled_var = (((n1 - 1) * (s1**2)) + ((n2 - 1) * (s2**2))) / (n1 + n2 - 2)
    pooled_sd = math.sqrt(pooled_var) if pooled_var > 0 else 0.0
    if pooled_sd == 0:
        return 0.0
    return (m1 - m2) / pooled_sd


def welch_df(a: List[float], b: List[float]) -> float:
    n1 = len(a)
    n2 = len(b)
    if n1 < 2 or n2 < 2:
        return 0.0
    s1 = stddev(a)
    s2 = stddev(b)
    v1 = (s1**2) / n1
    v2 = (s2**2) / n2
    num = (v1 + v2) ** 2
    den = ((v1**2) / (n1 - 1)) + ((v2**2) / (n2 - 1))
    if den == 0:
        return 0.0
    return num / den


def load_raw_rows(raw_path: Path) -> List[Dict[str, object]]:
    rows: List[Dict[str, object]] = []
    with raw_path.open("r", encoding="utf-8", newline="") as fp:
        reader = csv.DictReader(fp)
        for row in reader:
            rows.append(
                {
                    "strategy": row["strategy"],
                    "strategy_label": row.get("strategy_label", STRATEGY_LABELS[row["strategy"]]),
                    "concurrency": int(row["concurrency"]),
                    "run_id": int(row["run_id"]),
                    "throughput": float(row["throughput"]),
                    "p99": float(row["p99"]),
                    "median_latency": float(row.get("median_latency", 0.0)),
                    "avg_latency": float(row.get("avg_latency", 0.0)),
                    "latency_stddev": float(row.get("latency_stddev", 0.0)),
                    "latency_cv": float(row.get("latency_cv", 0.0)),
                    "errors": int(row.get("errors", 0)),
                    "warmup_ops_per_client": int(row.get("warmup_ops_per_client", 0)),
                    "measured_ops_per_client": int(row.get("measured_ops_per_client", 0)),
                }
            )
    return rows


def group_rows(rows: List[Dict[str, object]]) -> Dict[Tuple[str, int], List[Dict[str, object]]]:
    grouped: Dict[Tuple[str, int], List[Dict[str, object]]] = defaultdict(list)
    for row in rows:
        grouped[(str(row["strategy"]), int(row["concurrency"]))].append(row)

    for key in grouped:
        grouped[key].sort(key=lambda r: int(r["run_id"]))

    return grouped


def parse_cmdstat_file(path: Path) -> Tuple[Dict[str, str], List[str]]:
    metrics: Dict[str, str] = {}
    lines = path.read_text(encoding="utf-8").splitlines()
    for line in lines:
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        k, v = line.split(":", 1)
        metrics[k.strip()] = v.strip()
    return metrics, lines


def to_int(value: str, default: int = 0) -> int:
    try:
        return int(value)
    except Exception:
        return default


def to_float(value: str, default: float = 0.0) -> float:
    try:
        return float(value)
    except Exception:
        return default


def load_duration_seconds(bench_json: Path) -> float:
    payload = json.loads(bench_json.read_text(encoding="utf-8"))
    results = payload.get("results", [])
    if not results:
        return 0.0
    result = results[0]
    per_run = result.get("per_run", [])
    if per_run:
        return float(per_run[0].get("duration_secs", 0.0))
    return float(result.get("duration_secs", 0.0))


def relative(root: Path, path: Path) -> str:
    return str(path.resolve().relative_to(root.resolve()))


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()

    raw_path = (root / args.raw_data).resolve()
    metadata_path = (root / args.metadata).resolve()
    validation_latest_path = (root / args.validation_latest).resolve()

    output_json_path = (root / args.output_json).resolve()
    output_md_path = (root / args.output_md).resolve()
    config_output_path = (root / args.config_output).resolve()

    raw_rows = load_raw_rows(raw_path)
    grouped = group_rows(raw_rows)

    # Section 1: Raw data
    section_1_raw_data: List[Dict[str, object]] = []
    for strategy in STRATEGY_ORDER:
        for concurrency in CONCURRENCY_LEVELS:
            cfg_rows = grouped[(strategy, concurrency)]
            throughput_runs = [float(r["throughput"]) for r in cfg_rows]
            p99_runs = [float(r["p99"]) for r in cfg_rows]
            section_1_raw_data.append(
                {
                    "strategy": STRATEGY_LABELS[strategy],
                    "strategy_key": strategy,
                    "concurrency": concurrency,
                    "run_ids": [int(r["run_id"]) for r in cfg_rows],
                    "throughput_runs": throughput_runs,
                    "p99_runs": p99_runs,
                }
            )

    # Section 2: Aggregated stats
    section_2_aggregated_stats: List[Dict[str, object]] = []
    agg_lookup: Dict[Tuple[str, int], Dict[str, object]] = {}

    for strategy in STRATEGY_ORDER:
        for concurrency in CONCURRENCY_LEVELS:
            cfg_rows = grouped[(strategy, concurrency)]
            throughput_values = [float(r["throughput"]) for r in cfg_rows]
            p99_values = [float(r["p99"]) for r in cfg_rows]

            th_ci_low, th_ci_high = ci95(throughput_values)
            p99_ci_low, p99_ci_high = ci95(p99_values)

            row = {
                "strategy": STRATEGY_LABELS[strategy],
                "strategy_key": strategy,
                "concurrency": concurrency,
                "throughput": {
                    "n": len(throughput_values),
                    "mean": mean(throughput_values),
                    "stddev": stddev(throughput_values),
                    "CI_low": th_ci_low,
                    "CI_high": th_ci_high,
                    "CV": cv(throughput_values),
                },
                "p99": {
                    "n": len(p99_values),
                    "mean": mean(p99_values),
                    "stddev": stddev(p99_values),
                    "CI_low": p99_ci_low,
                    "CI_high": p99_ci_high,
                    "CV": cv(p99_values),
                },
            }
            section_2_aggregated_stats.append(row)
            agg_lookup[(strategy, concurrency)] = row

    # Section 4: Shapiro-Wilk
    section_4_shapiro_wilk: List[Dict[str, object]] = []
    shapiro_lookup: Dict[Tuple[str, int, str], Dict[str, object]] = {}

    for strategy in STRATEGY_ORDER:
        for concurrency in SHAPIRO_CONCURRENCY_LEVELS:
            cfg_rows = grouped[(strategy, concurrency)]
            for metric_name in ("throughput", "p99"):
                vals = [float(r[metric_name]) for r in cfg_rows]
                w_stat, p_value = stats.shapiro(vals)
                entry = {
                    "strategy": STRATEGY_LABELS[strategy],
                    "strategy_key": strategy,
                    "concurrency": concurrency,
                    "metric": metric_name,
                    "W": float(w_stat),
                    "p_value": float(p_value),
                    "is_normal": bool(p_value >= 0.05),
                }
                section_4_shapiro_wilk.append(entry)
                shapiro_lookup[(strategy, concurrency, metric_name)] = entry

    # Section 3: Statistical tests
    section_3_statistical_tests: List[Dict[str, object]] = []

    for concurrency in TEST_CONCURRENCY_LEVELS:
        for a_key, b_key, comp_label in COMPARISONS:
            for metric_name in ("throughput", "p99"):
                a_vals = [float(r[metric_name]) for r in grouped[(a_key, concurrency)]]
                b_vals = [float(r[metric_name]) for r in grouped[(b_key, concurrency)]]

                normal_a = shapiro_lookup[(a_key, concurrency, metric_name)]["is_normal"]
                normal_b = shapiro_lookup[(b_key, concurrency, metric_name)]["is_normal"]
                use_welch = bool(normal_a and normal_b)

                if use_welch:
                    t_result = stats.ttest_ind(a_vals, b_vals, equal_var=False)
                    test_name = "Welch t-test"
                    p_value = float(t_result.pvalue)
                    statistic = {
                        "t_statistic": float(t_result.statistic),
                        "degrees_of_freedom": float(welch_df(a_vals, b_vals)),
                    }
                    effect = {
                        "name": "cohen_d",
                        "value": float(cohen_d(a_vals, b_vals)),
                    }
                else:
                    u_result = stats.mannwhitneyu(a_vals, b_vals, alternative="two-sided")
                    u_value = float(u_result.statistic)
                    n1 = len(a_vals)
                    n2 = len(b_vals)
                    rank_biserial = ((2.0 * u_value) / (n1 * n2)) - 1.0
                    test_name = "Mann-Whitney U"
                    p_value = float(u_result.pvalue)
                    statistic = {
                        "u_statistic": u_value,
                    }
                    effect = {
                        "name": "rank_biserial_r",
                        "value": float(rank_biserial),
                    }

                section_3_statistical_tests.append(
                    {
                        "comparison": comp_label,
                        "group_a": STRATEGY_LABELS[a_key],
                        "group_a_key": a_key,
                        "group_b": STRATEGY_LABELS[b_key],
                        "group_b_key": b_key,
                        "concurrency": concurrency,
                        "metric": metric_name,
                        "normality": {
                            "group_a_is_normal": bool(normal_a),
                            "group_b_is_normal": bool(normal_b),
                        },
                        "test": test_name,
                        "statistic": statistic,
                        "p_value": p_value,
                        "effect_size": effect,
                        "sample_sizes": {"group_a": len(a_vals), "group_b": len(b_vals)},
                    }
                )

    # Section 5: System insights (raw)
    validation_run_text = validation_latest_path.read_text(encoding="utf-8").strip()
    validation_run_dir = (root / validation_run_text).resolve()

    sharded_n_cmd, sharded_n_lines = parse_cmdstat_file(
        validation_run_dir / "sharded_n_c500" / "cmdstat.txt"
    )
    sharded_2key_cmd, _ = parse_cmdstat_file(
        validation_run_dir / "sharded_2key_c500" / "cmdstat.txt"
    )
    hdr_400_cmd, _ = parse_cmdstat_file(
        validation_run_dir / "hdr_histogram_c400" / "cmdstat.txt"
    )
    hdr_500_cmd, _ = parse_cmdstat_file(
        validation_run_dir / "hdr_histogram_c500" / "cmdstat.txt"
    )
    thread_local_cmd, _ = parse_cmdstat_file(
        validation_run_dir / "thread_local_c500" / "cmdstat.txt"
    )

    shard_count_n = to_int(sharded_n_cmd.get("sharded_n_shard_count", "0"))
    sharded_n_shards: List[Dict[str, object]] = []
    for idx in range(shard_count_n):
        sharded_n_shards.append(
            {
                "shard": idx,
                "keys": to_int(sharded_n_cmd.get(f"sharded_n_shard_{idx}_keys", "0")),
                "calls": to_int(sharded_n_cmd.get(f"sharded_n_shard_{idx}_calls", "0")),
            }
        )

    shard_count_2key = to_int(sharded_2key_cmd.get("sharded_2key_shard_count", "0"))
    sharded_2key_shards: List[Dict[str, object]] = []
    for idx in range(shard_count_2key):
        sharded_2key_shards.append(
            {
                "shard": idx,
                "calls": to_int(sharded_2key_cmd.get(f"sharded_2key_shard_{idx}_calls", "0")),
            }
        )

    hdr_400_duration = load_duration_seconds(
        validation_run_dir / "hdr_histogram_c400" / "bench" / "benchmark_results.json"
    )
    hdr_500_duration = load_duration_seconds(
        validation_run_dir / "hdr_histogram_c500" / "bench" / "benchmark_results.json"
    )

    def hdr_row(cmd: Dict[str, str], concurrency: int, duration_secs: float) -> Dict[str, object]:
        phase_swaps = to_int(cmd.get("hdr_histogram_phase_swaps", "0"))
        cas_retries = to_int(cmd.get("hdr_histogram_cas_retries", "0"))
        return {
            "concurrency": concurrency,
            "phase_swaps": phase_swaps,
            "cas_retries": cas_retries,
            "count_trigger_hits": to_int(cmd.get("hdr_histogram_count_trigger_hits", "0")),
            "timer_trigger_hits": to_int(cmd.get("hdr_histogram_timer_trigger_hits", "0")),
            "bench_duration_secs": duration_secs,
            "phase_swap_frequency_hz": (phase_swaps / duration_secs) if duration_secs > 0 else 0.0,
        }

    bench_key_metric_lines = [
        line for line in sharded_n_lines if line.startswith("cmdstat_bench_key_")
    ]

    section_5_system_insights = {
        "system_validation_run_dir": relative(root, validation_run_dir),
        "sharded_n": {
            "shard_count": shard_count_n,
            "nonempty_shards": to_int(sharded_n_cmd.get("sharded_n_nonempty_shards", "0")),
            "cmdstat_truncated_entries": to_int(sharded_n_cmd.get("cmdstat_truncated_entries", "0")),
            "bench_key_metric_line_count": len(bench_key_metric_lines),
            "bench_key_metric_line_samples": bench_key_metric_lines[:20],
            "shard_distribution": sharded_n_shards,
        },
        "sharded_2key": {
            "shard_count": shard_count_2key,
            "get_shard": to_int(sharded_2key_cmd.get("sharded_2key_get_shard", "-1"), -1),
            "set_shard": to_int(sharded_2key_cmd.get("sharded_2key_set_shard", "-1"), -1),
            "shard_contention_calls": sharded_2key_shards,
        },
        "hdr_histogram": [
            hdr_row(hdr_400_cmd, 400, hdr_400_duration),
            hdr_row(hdr_500_cmd, 500, hdr_500_duration),
        ],
        "thread_local": {
            "concurrency": 500,
            "count_trigger_hits": to_int(thread_local_cmd.get("thread_local_count_trigger_hits", "0")),
            "timer_trigger_hits": to_int(thread_local_cmd.get("thread_local_timer_trigger_hits", "0")),
            "flush_with_batches": to_int(thread_local_cmd.get("thread_local_flush_with_batches", "0")),
            "count_based_threshold": 1000,
            "count_based_threshold_fired": to_int(thread_local_cmd.get("thread_local_count_trigger_hits", "0")) > 0,
        },
        "raw_log_paths": {
            "sharded_n_cmdstat": relative(root, validation_run_dir / "sharded_n_c500" / "cmdstat.txt"),
            "sharded_2key_cmdstat": relative(root, validation_run_dir / "sharded_2key_c500" / "cmdstat.txt"),
            "hdr_c400_cmdstat": relative(root, validation_run_dir / "hdr_histogram_c400" / "cmdstat.txt"),
            "hdr_c500_cmdstat": relative(root, validation_run_dir / "hdr_histogram_c500" / "cmdstat.txt"),
            "thread_local_cmdstat": relative(root, validation_run_dir / "thread_local_c500" / "cmdstat.txt"),
        },
    }

    # Section 6: Missing table data (Table 5.4.3)
    section_6_table_5_4_3: List[Dict[str, object]] = []
    for strategy in STRATEGY_ORDER:
        for concurrency in LOW_CONCURRENCY_LEVELS:
            agg = agg_lookup[(strategy, concurrency)]
            section_6_table_5_4_3.append(
                {
                    "strategy": STRATEGY_LABELS[strategy],
                    "strategy_key": strategy,
                    "concurrency": concurrency,
                    "throughput_mean": agg["throughput"]["mean"],
                    "throughput_stddev": agg["throughput"]["stddev"],
                    "throughput_CI_low": agg["throughput"]["CI_low"],
                    "throughput_CI_high": agg["throughput"]["CI_high"],
                    "throughput_CV": agg["throughput"]["CV"],
                    "p99_mean": agg["p99"]["mean"],
                    "p99_stddev": agg["p99"]["stddev"],
                    "p99_CI_low": agg["p99"]["CI_low"],
                    "p99_CI_high": agg["p99"]["CI_high"],
                    "p99_CV": agg["p99"]["CV"],
                }
            )

    # Figure datasets
    throughput_vs_concurrency: List[Dict[str, object]] = []
    p99_vs_concurrency: List[Dict[str, object]] = []
    cv_vs_concurrency: List[Dict[str, object]] = []

    for strategy in STRATEGY_ORDER:
        th_points: List[Dict[str, object]] = []
        p99_points: List[Dict[str, object]] = []
        cv_points: List[Dict[str, object]] = []
        for concurrency in CONCURRENCY_LEVELS:
            agg = agg_lookup[(strategy, concurrency)]
            th_points.append(
                {
                    "concurrency": concurrency,
                    "mean": agg["throughput"]["mean"],
                    "stddev": agg["throughput"]["stddev"],
                    "CI_low": agg["throughput"]["CI_low"],
                    "CI_high": agg["throughput"]["CI_high"],
                }
            )
            p99_points.append(
                {
                    "concurrency": concurrency,
                    "mean": agg["p99"]["mean"],
                    "stddev": agg["p99"]["stddev"],
                    "CI_low": agg["p99"]["CI_low"],
                    "CI_high": agg["p99"]["CI_high"],
                }
            )
            cv_points.append(
                {
                    "concurrency": concurrency,
                    "throughput_CV": agg["throughput"]["CV"],
                    "p99_CV": agg["p99"]["CV"],
                }
            )

        throughput_vs_concurrency.append(
            {
                "strategy": STRATEGY_LABELS[strategy],
                "strategy_key": strategy,
                "points": th_points,
            }
        )
        p99_vs_concurrency.append(
            {
                "strategy": STRATEGY_LABELS[strategy],
                "strategy_key": strategy,
                "points": p99_points,
            }
        )
        cv_vs_concurrency.append(
            {
                "strategy": STRATEGY_LABELS[strategy],
                "strategy_key": strategy,
                "points": cv_points,
            }
        )

    distribution_at_500_clients: List[Dict[str, object]] = []
    for strategy in STRATEGY_ORDER:
        cfg_rows = grouped[(strategy, 500)]
        distribution_at_500_clients.append(
            {
                "strategy": STRATEGY_LABELS[strategy],
                "strategy_key": strategy,
                "throughput_runs": [float(r["throughput"]) for r in cfg_rows],
                "p99_runs": [float(r["p99"]) for r in cfg_rows],
            }
        )

    # Section 7: Reproducibility metadata
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))

    config_output_path.parent.mkdir(parents=True, exist_ok=True)
    config_payload = {
        "matrix_source": {
            "raw_data_csv": relative(root, raw_path),
            "metadata_json": relative(root, metadata_path),
            "strategies": [STRATEGY_LABELS[s] for s in STRATEGY_ORDER],
            "concurrency_levels": CONCURRENCY_LEVELS,
            "runs_per_configuration": 30,
            "warmup_ops_per_client": 100,
            "measured_ops_per_client": 900,
            "workload": "50% GET / 50% SET",
            "keyspace": 10000,
            "value_size": 64,
        },
        "system_validation_source": {
            "run_dir": relative(root, validation_run_dir),
            "strategies": [
                "sharded_n@500",
                "sharded_2key@500",
                "hdr_histogram@400",
                "hdr_histogram@500",
                "thread_local@500",
            ],
        },
    }
    config_output_path.write_text(json.dumps(config_payload, indent=2) + "\n", encoding="utf-8")

    section_7_reproducibility_metadata = {
        "os_version": metadata.get("machine_specs", {}).get("platform", ""),
        "cpu_info": metadata.get("machine_specs", {}).get("cpu_brand", ""),
        "logical_threads": metadata.get("machine_specs", {}).get("logicalcpu", ""),
        "physical_threads": metadata.get("machine_specs", {}).get("physicalcpu", ""),
        "runtime": metadata.get("runtime_config", {}).get("runtime", ""),
        "runtime_version": metadata.get("machine_specs", {}).get("rustc", ""),
        "python_version": metadata.get("machine_specs", {}).get("python_version", ""),
        "commit_hash": metadata.get("machine_specs", {}).get("commit_hash", ""),
        "worker_threads": metadata.get("runtime_config", {}).get("worker_threads", ""),
        "config_file": relative(root, config_output_path),
    }

    final_payload = {
        "dataset_name": "final_experiment_v12",
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source_artifacts": {
            "raw_data_csv": relative(root, raw_path),
            "metadata_json": relative(root, metadata_path),
            "system_validation_latest": relative(root, validation_latest_path),
            "system_validation_run_dir": relative(root, validation_run_dir),
        },
        "sections": {
            "section_1_raw_data": section_1_raw_data,
            "section_2_aggregated_stats": section_2_aggregated_stats,
            "section_3_statistical_tests": section_3_statistical_tests,
            "section_4_shapiro_wilk": section_4_shapiro_wilk,
            "section_5_system_insights_raw": section_5_system_insights,
            "section_6_missing_table_data_table_5_4_3": section_6_table_5_4_3,
            "section_7_reproducibility_metadata": section_7_reproducibility_metadata,
        },
        "figure_datasets": {
            "throughput_vs_concurrency": throughput_vs_concurrency,
            "p99_vs_concurrency": p99_vs_concurrency,
            "cv_vs_concurrency": cv_vs_concurrency,
            "distribution_at_500_clients": distribution_at_500_clients,
        },
    }

    output_json_path.write_text(json.dumps(final_payload, indent=2) + "\n", encoding="utf-8")

    # Markdown summary
    md_lines: List[str] = []
    md_lines.append("# final_experiment_summary")
    md_lines.append("")
    md_lines.append(f"Generated at: {final_payload['generated_at_utc']}")
    md_lines.append("")
    md_lines.append("## Artifact Paths")
    md_lines.append("")
    md_lines.append(f"- JSON: {relative(root, output_json_path)}")
    md_lines.append(f"- Raw matrix source: {relative(root, raw_path)}")
    md_lines.append(f"- Metadata source: {relative(root, metadata_path)}")
    md_lines.append(f"- System validation source: {relative(root, validation_run_dir)}")
    md_lines.append(f"- Config file: {relative(root, config_output_path)}")
    md_lines.append("")
    md_lines.append("## Matrix Coverage")
    md_lines.append("")
    md_lines.append(f"- Strategies: {', '.join(STRATEGY_LABELS[s] for s in STRATEGY_ORDER)}")
    md_lines.append(f"- Concurrency levels: {', '.join(str(x) for x in CONCURRENCY_LEVELS)}")
    md_lines.append("- Runs per configuration: 30")
    md_lines.append("- Warmup/measured per client: 100 / 900")
    md_lines.append("- Workload: 50% GET / 50% SET")
    md_lines.append("- Keyspace/value size: 10,000 / 64 bytes")
    md_lines.append("")
    md_lines.append("## Section Checklist")
    md_lines.append("")
    md_lines.append(f"- Section 1 raw configs: {len(section_1_raw_data)}")
    md_lines.append(f"- Section 2 aggregated configs: {len(section_2_aggregated_stats)}")
    md_lines.append(f"- Section 3 statistical tests: {len(section_3_statistical_tests)}")
    md_lines.append(f"- Section 4 Shapiro-Wilk rows: {len(section_4_shapiro_wilk)}")
    md_lines.append("- Section 5 system insights: included")
    md_lines.append(f"- Section 6 Table 5.4.3 rows: {len(section_6_table_5_4_3)}")
    md_lines.append("- Section 7 reproducibility metadata: included")
    md_lines.append("")
    md_lines.append("## Primary Comparisons (500/600/700; throughput + p99)")
    md_lines.append("")
    md_lines.append("| Comparison | Concurrency | Metric | Test | p-value | Effect |")
    md_lines.append("|---|---:|---|---|---:|---:|")
    for row in section_3_statistical_tests:
        comp = row["comparison"]
        conc = row["concurrency"]
        metric = row["metric"]
        test_name = row["test"]
        p_value = row["p_value"]
        effect = row["effect_size"]["value"]
        md_lines.append(
            f"| {comp} | {conc} | {metric} | {test_name} | {p_value:.6g} | {effect:.6g} |"
        )

    md_lines.append("")
    md_lines.append("## Shapiro-Wilk (400/500/600/700)")
    md_lines.append("")
    md_lines.append("| Strategy | Concurrency | Metric | W | p-value | Normal (p>=0.05) |")
    md_lines.append("|---|---:|---|---:|---:|---|")
    for row in section_4_shapiro_wilk:
        md_lines.append(
            f"| {row['strategy']} | {row['concurrency']} | {row['metric']} | {row['W']:.6g} | {row['p_value']:.6g} | {row['is_normal']} |"
        )

    output_md_path.write_text("\n".join(md_lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
