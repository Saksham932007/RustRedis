#!/usr/bin/env python3
"""
RustRedis Benchmark Analysis & Graph Generator
================================================
Reads benchmark_results.json and produces publication-quality graphs.

Usage:
    python3 benchmarks/analysis.py [--input benchmarks/results/benchmark_results.json] [--output benchmarks/results/]

Dependencies:
    pip install matplotlib numpy
"""

import json
import os
import sys
import argparse
from pathlib import Path

try:
    import matplotlib
    matplotlib.use('Agg')  # Non-interactive backend
    import matplotlib.pyplot as plt
    import matplotlib.ticker as ticker
    import numpy as np
except ImportError:
    print("ERROR: matplotlib and numpy are required.")
    print("Install with: pip install matplotlib numpy")
    sys.exit(1)

# ── Style Configuration ─────────────────────────────────────────────────────

COLORS = {
    'RustRedis': '#E74C3C',
    'Redis': '#3498DB',
    'read_heavy': '#2ECC71',
    'write_heavy': '#E67E22',
    'mixed': '#9B59B6',
}

plt.rcParams.update({
    'figure.facecolor': '#1a1a2e',
    'axes.facecolor': '#16213e',
    'axes.edgecolor': '#e0e0e0',
    'axes.labelcolor': '#e0e0e0',
    'text.color': '#e0e0e0',
    'xtick.color': '#e0e0e0',
    'ytick.color': '#e0e0e0',
    'grid.color': '#2a2a4a',
    'grid.alpha': 0.5,
    'font.family': 'sans-serif',
    'font.size': 11,
    'legend.facecolor': '#16213e',
    'legend.edgecolor': '#e0e0e0',
    'legend.labelcolor': '#e0e0e0',
})


def load_results(path):
    """Load benchmark results from JSON file."""
    with open(path, 'r') as f:
        return json.load(f)


def get_workload_results(results, target, workload_name):
    """Filter results by target and workload name substring."""
    return sorted(
        [r for r in results if r['target'] == target and workload_name.lower() in r['name'].lower()],
        key=lambda r: r['concurrency']
    )


def plot_throughput_vs_concurrency(results, output_dir):
    """Generate throughput vs concurrency scaling graph."""
    fig, ax = plt.subplots(figsize=(12, 7))

    workloads = [
        ('Read-Heavy', 'read_heavy', COLORS['read_heavy']),
        ('Write-Heavy', 'write_heavy', COLORS['write_heavy']),
        ('Mixed', 'mixed', COLORS['mixed']),
    ]

    targets = list(set(r['target'] for r in results))

    for target in targets:
        linestyle = '-' if target == 'RustRedis' else '--'
        marker = 'o' if target == 'RustRedis' else 's'

        for wl_name, wl_key, color in workloads:
            data = get_workload_results(results, target, wl_name)
            if not data:
                continue

            concurrencies = [d['concurrency'] for d in data]
            throughputs = [d['ops_per_sec'] for d in data]

            ax.plot(concurrencies, throughputs,
                    marker=marker, linestyle=linestyle, color=color,
                    linewidth=2, markersize=6,
                    label=f'{target} - {wl_name}')

    ax.set_xlabel('Concurrent Clients', fontsize=13, fontweight='bold')
    ax.set_ylabel('Throughput (ops/sec)', fontsize=13, fontweight='bold')
    ax.set_title('Throughput vs Concurrency Scaling', fontsize=16, fontweight='bold', pad=15)
    ax.set_xscale('log')
    ax.legend(loc='best', fontsize=9, framealpha=0.8)
    ax.grid(True, alpha=0.3)
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))

    plt.tight_layout()
    path = os.path.join(output_dir, 'throughput_vs_concurrency.png')
    fig.savefig(path, dpi=150, bbox_inches='tight')
    plt.close(fig)
    print(f'  ✓ {path}')


def plot_latency_vs_concurrency(results, output_dir):
    """Generate latency percentiles vs concurrency graph."""
    fig, axes = plt.subplots(1, 3, figsize=(18, 6))

    workloads = [
        ('Read-Heavy', 'read_heavy'),
        ('Write-Heavy', 'write_heavy'),
        ('Mixed', 'mixed'),
    ]

    percentiles = [('p50', 'p50_us'), ('p95', 'p95_us'), ('p99', 'p99_us')]
    percentile_colors = ['#2ECC71', '#F39C12', '#E74C3C']

    targets = list(set(r['target'] for r in results))

    for idx, (wl_name, wl_key) in enumerate(workloads):
        ax = axes[idx]

        for target in targets:
            linestyle = '-' if target == 'RustRedis' else '--'

            data = get_workload_results(results, target, wl_name)
            if not data:
                continue

            concurrencies = [d['concurrency'] for d in data]

            for (p_name, p_key), p_color in zip(percentiles, percentile_colors):
                values = [d[p_key] for d in data]
                marker = 'o' if target == 'RustRedis' else 's'
                ax.plot(concurrencies, values,
                        marker=marker, linestyle=linestyle, color=p_color,
                        linewidth=1.5, markersize=5,
                        label=f'{target} {p_name}')

        ax.set_xlabel('Concurrent Clients', fontsize=11)
        ax.set_ylabel('Latency (µs)', fontsize=11)
        ax.set_title(f'{wl_name} Latency', fontsize=13, fontweight='bold')
        ax.set_xscale('log')
        ax.set_yscale('log')
        ax.legend(loc='best', fontsize=7, framealpha=0.8)
        ax.grid(True, alpha=0.3)

    plt.suptitle('Latency Distribution vs Concurrency', fontsize=16, fontweight='bold', y=1.02)
    plt.tight_layout()
    path = os.path.join(output_dir, 'latency_vs_concurrency.png')
    fig.savefig(path, dpi=150, bbox_inches='tight')
    plt.close(fig)
    print(f'  ✓ {path}')


def plot_memory_over_time(memory_samples, output_dir):
    """Generate memory usage over time graph."""
    if not memory_samples:
        return

    fig, ax = plt.subplots(figsize=(12, 6))

    times = [s['elapsed_secs'] for s in memory_samples]
    rss_mb = [s['rss_bytes'] / (1024 * 1024) for s in memory_samples]
    vsize_mb = [s['vsize_bytes'] / (1024 * 1024) for s in memory_samples]

    ax.plot(times, rss_mb, 'o-', color='#E74C3C', linewidth=2, markersize=6, label='RSS (Resident)')
    ax.fill_between(times, rss_mb, alpha=0.2, color='#E74C3C')
    ax.plot(times, vsize_mb, 's--', color='#3498DB', linewidth=1.5, markersize=5, label='Virtual Size')

    # Annotate points
    for i, sample in enumerate(memory_samples):
        if i % max(1, len(memory_samples) // 8) == 0:
            ax.annotate(sample['label'], (times[i], rss_mb[i]),
                       textcoords="offset points", xytext=(0, 10),
                       fontsize=7, ha='center', color='#e0e0e0')

    ax.set_xlabel('Time (seconds)', fontsize=13, fontweight='bold')
    ax.set_ylabel('Memory (MB)', fontsize=13, fontweight='bold')
    ax.set_title('Memory Usage During Benchmark', fontsize=16, fontweight='bold', pad=15)
    ax.legend(loc='best', fontsize=10, framealpha=0.8)
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    path = os.path.join(output_dir, 'memory_vs_time.png')
    fig.savefig(path, dpi=150, bbox_inches='tight')
    plt.close(fig)
    print(f'  ✓ {path}')


def plot_comparison_bar(results, output_dir):
    """Generate side-by-side bar chart comparing RustRedis vs Redis."""
    rust_results = [r for r in results if r['target'] == 'RustRedis']
    redis_results = [r for r in results if r['target'] == 'Redis']

    if not redis_results:
        return

    # Find matching concurrency levels
    common_conc = sorted(set(
        r['concurrency'] for r in rust_results
    ).intersection(
        r['concurrency'] for r in redis_results
    ))

    if not common_conc:
        return

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    x = np.arange(len(common_conc))
    width = 0.35

    # Throughput comparison (using mixed workload)
    rust_tp = []
    redis_tp = []
    for c in common_conc:
        rd = [r for r in rust_results if r['concurrency'] == c and 'mixed' in r['name'].lower()]
        re = [r for r in redis_results if r['concurrency'] == c and 'mixed' in r['name'].lower()]
        rust_tp.append(rd[0]['ops_per_sec'] if rd else 0)
        redis_tp.append(re[0]['ops_per_sec'] if re else 0)

    bars1 = ax1.bar(x - width/2, rust_tp, width, label='RustRedis', color=COLORS['RustRedis'], alpha=0.85)
    bars2 = ax1.bar(x + width/2, redis_tp, width, label='Redis', color=COLORS['Redis'], alpha=0.85)
    ax1.set_xlabel('Concurrent Clients')
    ax1.set_ylabel('Throughput (ops/sec)')
    ax1.set_title('Throughput Comparison (Mixed Workload)', fontweight='bold')
    ax1.set_xticks(x)
    ax1.set_xticklabels(common_conc)
    ax1.legend()
    ax1.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))

    # Latency comparison
    rust_p99 = []
    redis_p99 = []
    for c in common_conc:
        rd = [r for r in rust_results if r['concurrency'] == c and 'mixed' in r['name'].lower()]
        re = [r for r in redis_results if r['concurrency'] == c and 'mixed' in r['name'].lower()]
        rust_p99.append(rd[0]['p99_us'] if rd else 0)
        redis_p99.append(re[0]['p99_us'] if re else 0)

    ax2.bar(x - width/2, rust_p99, width, label='RustRedis', color=COLORS['RustRedis'], alpha=0.85)
    ax2.bar(x + width/2, redis_p99, width, label='Redis', color=COLORS['Redis'], alpha=0.85)
    ax2.set_xlabel('Concurrent Clients')
    ax2.set_ylabel('p99 Latency (µs)')
    ax2.set_title('p99 Latency Comparison (Mixed Workload)', fontweight='bold')
    ax2.set_xticks(x)
    ax2.set_xticklabels(common_conc)
    ax2.legend()

    plt.tight_layout()
    path = os.path.join(output_dir, 'comparison_rustredis_vs_redis.png')
    fig.savefig(path, dpi=150, bbox_inches='tight')
    plt.close(fig)
    print(f'  ✓ {path}')


def generate_markdown_table(results):
    """Generate a markdown summary table."""
    lines = [
        "| Target | Workload | Concurrency | ops/sec | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) |",
        "|--------|----------|-------------|---------|----------|----------|----------|----------|",
    ]
    for r in sorted(results, key=lambda x: (x['target'], x['name'], x['concurrency'])):
        lines.append(
            f"| {r['target']} | {r['name'][:30]} | {r['concurrency']} | "
            f"{r['ops_per_sec']:,.0f} | {r['p50_us']:,.0f} | {r['p95_us']:,.0f} | "
            f"{r['p99_us']:,.0f} | {r['max_us']:,.0f} |"
        )
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description='RustRedis Benchmark Analysis')
    parser.add_argument('--input', default='benchmarks/results/benchmark_results.json',
                       help='Path to benchmark results JSON')
    parser.add_argument('--output', default='benchmarks/results/',
                       help='Output directory for graphs')
    args = parser.parse_args()

    if not os.path.exists(args.input):
        print(f"ERROR: Results file not found: {args.input}")
        print("Run the benchmark first: cd benchmarks && cargo run --release")
        sys.exit(1)

    os.makedirs(args.output, exist_ok=True)
    data = load_results(args.input)

    print("Generating performance graphs...")
    print()

    plot_throughput_vs_concurrency(data['results'], args.output)
    plot_latency_vs_concurrency(data['results'], args.output)
    plot_memory_over_time(data.get('memory_samples', []), args.output)
    plot_comparison_bar(data['results'], args.output)

    # Generate markdown table
    table = generate_markdown_table(data['results'])
    table_path = os.path.join(args.output, 'summary_table.md')
    with open(table_path, 'w') as f:
        f.write("# Benchmark Results Summary\n\n")
        f.write(table)
        f.write('\n')
    print(f'  ✓ {table_path}')

    print(f"\nDone! All graphs saved to {args.output}")


if __name__ == '__main__':
    main()
