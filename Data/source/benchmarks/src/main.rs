use clap::Parser;
use rand::Rng;
use serde::Serialize;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "rustredis-bench", about = "RustRedis Benchmark Suite")]
struct Args {
    /// Target host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Target port
    #[arg(long, default_value_t = 6379)]
    port: u16,

    /// Number of requests per client
    #[arg(long, default_value_t = 10000)]
    requests: u64,

    /// Number of concurrent clients
    #[arg(long, default_value = "1,10,100,500,1000")]
    concurrency: String,

    /// Value size in bytes
    #[arg(long, default_value_t = 64)]
    value_size: usize,

    /// Workload type: read-heavy, write-heavy, mixed, or all
    #[arg(long, default_value = "all")]
    workload: String,

    /// Output directory for results
    #[arg(long, default_value = "results")]
    output_dir: String,

    /// Run comparison against real Redis on this port (0 to skip)
    #[arg(long, default_value_t = 0)]
    redis_port: u16,

    /// Key space size (number of unique keys)
    #[arg(long, default_value_t = 10000)]
    key_space: u64,

    /// Number of runs per configuration for statistical averaging
    #[arg(long, default_value_t = 3)]
    runs: usize,

    /// Run metrics strategy comparison benchmark
    /// (server must be restarted with each RUSTREDIS_METRICS_STRATEGY for accurate results)
    #[arg(long, default_value = "none")]
    metrics_strategy: String,
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Serialize, Clone)]
struct BenchmarkResult {
    /// Name of the benchmark
    name: String,
    /// Number of concurrent clients
    concurrency: usize,
    /// Number of warmup requests ignored per client
    warmup_ops_per_client: u64,
    /// Number of measured requests per client
    measured_ops_per_client: u64,
    /// Total operations completed
    total_ops: u64,
    /// Duration in seconds
    duration_secs: f64,
    /// Throughput in ops/sec
    ops_per_sec: f64,
    /// Latency percentiles in microseconds
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    max_us: f64,
    avg_us: f64,
    latency_stddev_us: f64,
    latency_cv: f64,
    /// Errors encountered
    errors: u64,
    /// Target description
    target: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkSuite {
    timestamp: String,
    runs_per_config: usize,
    results: Vec<AggregatedResult>,
    memory_samples: Vec<MemorySample>,
}

#[derive(Debug, Serialize, Clone)]
struct AggregatedResult {
    name: String,
    concurrency: usize,
    target: String,
    runs: usize,
    ops_per_sec_mean: f64,
    ops_per_sec_stddev: f64,
    p50_us_mean: f64,
    p50_us_stddev: f64,
    p99_us_mean: f64,
    p99_us_stddev: f64,
    max_us_mean: f64,
    max_us_stddev: f64,
    total_errors: u64,
    per_run: Vec<BenchmarkResult>,
}

#[derive(Debug, Serialize, Clone)]
struct MemorySample {
    elapsed_secs: f64,
    rss_bytes: u64,
    vsize_bytes: u64,
    label: String,
}

// ============================================================================
// RESP Protocol — Minimal Client
// ============================================================================

struct RespClient {
    stream: TcpStream,
    buf: Vec<u8>,
}

impl RespClient {
    fn connect(host: &str, port: u16) -> io::Result<Self> {
        let stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.set_nodelay(true)?;
        // 5 second timeout
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(RespClient {
            stream,
            buf: vec![0u8; 65536],
        })
    }

    fn send_command(&mut self, args: &[&str]) -> io::Result<()> {
        // Build RESP array
        let mut cmd = format!("*{}\r\n", args.len());
        for arg in args {
            cmd.push_str(&format!("${}\r\n{}\r\n", arg.len(), arg));
        }
        self.stream.write_all(cmd.as_bytes())?;
        Ok(())
    }

    fn read_response(&mut self) -> io::Result<usize> {
        self.stream.read(&mut self.buf)
    }

    fn ping(&mut self) -> io::Result<bool> {
        self.send_command(&["PING"])?;
        let n = self.read_response()?;
        Ok(n > 0)
    }

    fn set(&mut self, key: &str, value: &str) -> io::Result<()> {
        self.send_command(&["SET", key, value])?;
        self.read_response()?;
        Ok(())
    }

    fn get(&mut self, key: &str) -> io::Result<()> {
        self.send_command(&["GET", key])?;
        self.read_response()?;
        Ok(())
    }

    fn flushdb(&mut self) -> io::Result<()> {
        self.send_command(&["FLUSHDB"])?;
        self.read_response()?;
        Ok(())
    }

    #[allow(dead_code)]
    fn set_with_ttl(&mut self, key: &str, value: &str, ttl_secs: u64) -> io::Result<()> {
        self.send_command(&["SET", key, value, "EX", &ttl_secs.to_string()])?;
        self.read_response()?;
        Ok(())
    }

    fn cmdstat(&mut self) -> io::Result<String> {
        self.send_command(&["CMDSTAT"])?;
        let n = self.read_response()?;
        Ok(String::from_utf8_lossy(&self.buf[..n]).to_string())
    }
}

// ============================================================================
// Workload Definitions
// ============================================================================

#[derive(Clone, Copy, Debug)]
enum WorkloadType {
    ReadHeavy,   // 80% GET, 20% SET
    WriteHeavy,  // 80% SET, 20% GET
    Mixed,       // 50% GET, 50% SET
}

impl WorkloadType {
    fn name(&self) -> &'static str {
        match self {
            WorkloadType::ReadHeavy => "read_heavy",
            WorkloadType::WriteHeavy => "write_heavy",
            WorkloadType::Mixed => "mixed",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            WorkloadType::ReadHeavy => "Read-Heavy (80% GET / 20% SET)",
            WorkloadType::WriteHeavy => "Write-Heavy (80% SET / 20% GET)",
            WorkloadType::Mixed => "Mixed (50% GET / 50% SET)",
        }
    }

    fn read_ratio(&self) -> f64 {
        match self {
            WorkloadType::ReadHeavy => 0.8,
            WorkloadType::WriteHeavy => 0.2,
            WorkloadType::Mixed => 0.5,
        }
    }
}

// ============================================================================
// Latency Tracker (sorted vector for percentile computation)
// ============================================================================

struct LatencyTracker {
    samples: Vec<u64>, // microseconds
}

#[allow(dead_code)]
impl LatencyTracker {
    fn new(capacity: usize) -> Self {
        LatencyTracker {
            samples: Vec::with_capacity(capacity),
        }
    }

    fn record(&mut self, us: u64) {
        self.samples.push(us);
    }

    fn percentile(&mut self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.sort_unstable();
        let idx = ((p / 100.0) * (self.samples.len() as f64 - 1.0)) as usize;
        self.samples[idx.min(self.samples.len() - 1)] as f64
    }

    fn max(&self) -> f64 {
        self.samples.iter().copied().max().unwrap_or(0) as f64
    }

    fn avg(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.samples.iter().sum();
        sum as f64 / self.samples.len() as f64
    }
}

// ============================================================================
// Memory Sampling
// ============================================================================

fn sample_memory(label: &str, start: Instant) -> Option<MemorySample> {
    let contents = std::fs::read_to_string("/proc/self/statm").ok()?;
    let mut parts = contents.split_whitespace();
    let vsize_pages: u64 = parts.next()?.parse().ok()?;
    let rss_pages: u64 = parts.next()?.parse().ok()?;
    let page_size = 4096u64;
    Some(MemorySample {
        elapsed_secs: start.elapsed().as_secs_f64(),
        rss_bytes: rss_pages * page_size,
        vsize_bytes: vsize_pages * page_size,
        label: label.to_string(),
    })
}

/// Try to sample a remote Redis/RustRedis server's memory from the benchmark process perspective
fn sample_process_memory(start: Instant, label: &str) -> MemorySample {
    sample_memory(label, start).unwrap_or(MemorySample {
        elapsed_secs: start.elapsed().as_secs_f64(),
        rss_bytes: 0,
        vsize_bytes: 0,
        label: label.to_string(),
    })
}

// ============================================================================
// Core Benchmark Runner
// ============================================================================

fn run_single_workload(
    host: &str,
    port: u16,
    concurrency: usize,
    requests_per_client: u64,
    key_space: u64,
    value_size: usize,
    workload: WorkloadType,
    target_name: &str,
) -> BenchmarkResult {
    let value: String = "x".repeat(value_size);
    let total_ops = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));
    let warmup_barrier = Arc::new(Barrier::new(concurrency + 1));
    let warmup_ops_per_client = requests_per_client / 10;
    let measured_ops_per_client = requests_per_client.saturating_sub(warmup_ops_per_client);

    let all_latencies: Arc<std::sync::Mutex<Vec<u64>>> =
        Arc::new(std::sync::Mutex::new(Vec::with_capacity(
            (concurrency as u64 * measured_ops_per_client) as usize,
        )));

    // Use OS threads for blocking I/O (TcpStream is synchronous)
    let mut handles = Vec::new();
    for _ in 0..concurrency {
        let host = host.to_string();
        let value = value.clone();
        let total_ops = Arc::clone(&total_ops);
        let total_errors = Arc::clone(&total_errors);
        let all_latencies = Arc::clone(&all_latencies);
        let warmup_barrier = Arc::clone(&warmup_barrier);

        handles.push(std::thread::spawn(move || {
            let mut client = match RespClient::connect(&host, port) {
                Ok(c) => c,
                Err(_) => {
                    warmup_barrier.wait();
                    total_errors.fetch_add(measured_ops_per_client, Ordering::Relaxed);
                    return;
                }
            };

            let mut rng = rand::thread_rng();
            let mut local_latencies = Vec::with_capacity(measured_ops_per_client as usize);

            // Warmup phase: execute but do not include in metrics.
            for _ in 0..warmup_ops_per_client {
                let key = format!("bench:key:{}", rng.gen_range(0..key_space));
                let is_read = rng.gen::<f64>() < workload.read_ratio();

                let _ = if is_read {
                    client.get(&key)
                } else {
                    client.set(&key, &value)
                };
            }

            warmup_barrier.wait();

            for _ in 0..measured_ops_per_client {
                let key = format!("bench:key:{}", rng.gen_range(0..key_space));
                let is_read = rng.gen::<f64>() < workload.read_ratio();

                let op_start = Instant::now();
                let result = if is_read {
                    client.get(&key)
                } else {
                    client.set(&key, &value)
                };
                let elapsed_us = op_start.elapsed().as_micros() as u64;

                match result {
                    Ok(_) => {
                        total_ops.fetch_add(1, Ordering::Relaxed);
                        local_latencies.push(elapsed_us);
                    }
                    Err(_) => {
                        total_errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            all_latencies
                .lock()
                .unwrap()
                .extend_from_slice(&local_latencies);
        }));
    }

    // Start measured wall-clock timer once all clients complete warmup.
    warmup_barrier.wait();
    let measured_start = Instant::now();

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = measured_start.elapsed();
    let ops = total_ops.load(Ordering::Relaxed);
    let errors = total_errors.load(Ordering::Relaxed);

    let mut tracker = LatencyTracker {
        samples: Arc::try_unwrap(all_latencies)
            .unwrap()
            .into_inner()
            .unwrap(),
    };

    let latency_stddev_us = stddev_u64(&tracker.samples);
    let avg_us = tracker.avg();
    let latency_cv = if avg_us > 0.0 {
        latency_stddev_us / avg_us
    } else {
        0.0
    };
    let duration_secs = duration.as_secs_f64();
    let ops_per_sec = if duration_secs > 0.0 {
        ops as f64 / duration_secs
    } else {
        0.0
    };

    BenchmarkResult {
        name: format!("{}", workload.display_name()),
        concurrency,
        warmup_ops_per_client,
        measured_ops_per_client,
        total_ops: ops,
        duration_secs,
        ops_per_sec,
        p50_us: tracker.percentile(50.0),
        p95_us: tracker.percentile(95.0),
        p99_us: tracker.percentile(99.0),
        max_us: tracker.max(),
        avg_us,
        latency_stddev_us,
        latency_cv,
        errors,
        target: target_name.to_string(),
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64]) -> f64 {
    if values.len() < 2 { return 0.0; }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

fn stddev_u64(values: &[u64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let m = values.iter().map(|v| *v as f64).sum::<f64>() / n;
    let variance = values
        .iter()
        .map(|v| {
            let dv = *v as f64 - m;
            dv * dv
        })
        .sum::<f64>()
        / (n - 1.0);
    variance.sqrt()
}

fn aggregate_runs(runs: Vec<BenchmarkResult>) -> AggregatedResult {
    let n = runs.len();
    let ops: Vec<f64> = runs.iter().map(|r| r.ops_per_sec).collect();
    let p50: Vec<f64> = runs.iter().map(|r| r.p50_us).collect();
    let p99: Vec<f64> = runs.iter().map(|r| r.p99_us).collect();
    let maxl: Vec<f64> = runs.iter().map(|r| r.max_us).collect();
    let total_errors: u64 = runs.iter().map(|r| r.errors).sum();

    AggregatedResult {
        name: runs[0].name.clone(),
        concurrency: runs[0].concurrency,
        target: runs[0].target.clone(),
        runs: n,
        ops_per_sec_mean: mean(&ops),
        ops_per_sec_stddev: stddev(&ops),
        p50_us_mean: mean(&p50),
        p50_us_stddev: stddev(&p50),
        p99_us_mean: mean(&p99),
        p99_us_stddev: stddev(&p99),
        max_us_mean: mean(&maxl),
        max_us_stddev: stddev(&maxl),
        total_errors,
        per_run: runs,
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           RustRedis Benchmark Suite v1.0                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create output directory
    std::fs::create_dir_all(&args.output_dir).expect("Failed to create output directory");

    let concurrency_levels: Vec<usize> = args
        .concurrency
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let workloads: Vec<WorkloadType> = match args.workload.as_str() {
        "read-heavy" | "read" => vec![WorkloadType::ReadHeavy],
        "write-heavy" | "write" => vec![WorkloadType::WriteHeavy],
        "mixed" => vec![WorkloadType::Mixed],
        _ => vec![
            WorkloadType::ReadHeavy,
            WorkloadType::WriteHeavy,
            WorkloadType::Mixed,
        ],
    };

    let mut all_results: Vec<AggregatedResult> = Vec::new();
    let mut memory_samples: Vec<MemorySample> = Vec::new();
    let global_start = Instant::now();

    // Test connectivity
    print!("Testing connection to {}:{}... ", args.host, args.port);
    match RespClient::connect(&args.host, args.port) {
        Ok(mut client) => match client.ping() {
            Ok(true) => println!("✓ Connected"),
            _ => {
                println!("✗ Failed to PING");
                std::process::exit(1);
            }
        },
        Err(e) => {
            println!("✗ Connection failed: {}", e);
            println!("\nMake sure RustRedis is running: cargo run --bin server");
            std::process::exit(1);
        }
    }

    println!("Runs per configuration: {} (mean ± stddev)", args.runs);

    // Collect idle memory sample
    memory_samples.push(sample_process_memory(global_start, "idle"));

    // ── Run benchmarks ──────────────────────────────────────────────────────
    for workload in &workloads {
        println!("\n━━━ {} ━━━", workload.display_name());

        for &conc in &concurrency_levels {
            print!(
                "  {:>5} clients × {:>7} ops × {} runs ... ",
                conc, args.requests, args.runs
            );
            std::io::stdout().flush().ok();

            let mut run_results = Vec::new();
            for _ in 0..args.runs {
                // Flush DB before each run
                if let Ok(mut client) = RespClient::connect(&args.host, args.port) {
                    let _ = client.flushdb();
                }

                // Pre-populate for read-heavy workloads
                if matches!(workload, WorkloadType::ReadHeavy) {
                    if let Ok(mut client) = RespClient::connect(&args.host, args.port) {
                        let value = "x".repeat(args.value_size);
                        let populate_count = args.key_space.min(5000);
                        for i in 0..populate_count {
                            let _ = client.set(&format!("bench:key:{}", i), &value);
                        }
                    }
                }

                let result = run_single_workload(
                    &args.host,
                    args.port,
                    conc,
                    args.requests,
                    args.key_space,
                    args.value_size,
                    *workload,
                    "RustRedis",
                );
                run_results.push(result);
            }

            let agg = aggregate_runs(run_results);
            let cv = if agg.ops_per_sec_mean > 0.0 {
                agg.ops_per_sec_stddev / agg.ops_per_sec_mean * 100.0
            } else {
                0.0
            };
            println!(
                "{:>7.0} ± {:<5.0} ops/sec (CV={:.1}%) | p50={:.0}±{:.0}µs  p99={:.0}±{:.0}µs",
                agg.ops_per_sec_mean, agg.ops_per_sec_stddev, cv,
                agg.p50_us_mean, agg.p50_us_stddev,
                agg.p99_us_mean, agg.p99_us_stddev,
            );

            all_results.push(agg);

            // Memory sample after this workload
            memory_samples.push(sample_process_memory(
                global_start,
                &format!("{}_c{}", workload.name(), conc),
            ));
        }
    }

    // ── Redis comparison (if requested) ─────────────────────────────────────
    if args.redis_port > 0 {
        println!("\n━━━ Comparison: Real Redis (port {}) ━━━", args.redis_port);

        // Test Redis connectivity
        match RespClient::connect(&args.host, args.redis_port) {
            Ok(mut client) => match client.ping() {
                Ok(true) => println!("  ✓ Connected to Redis"),
                _ => {
                    println!("  ✗ Failed to PING Redis, skipping");
                    return write_results(&args.output_dir, args.runs, all_results, memory_samples);
                }
            },
            Err(e) => {
                println!("  ✗ Cannot connect to Redis: {}, skipping", e);
                return write_results(&args.output_dir, args.runs, all_results, memory_samples);
            }
        }

        for workload in &workloads {
            for &conc in &concurrency_levels {
                print!(
                    "  [Redis] {:>5} clients × {:>7} ops × {} runs ({}) ... ",
                    conc, args.requests, args.runs, workload.display_name()
                );
                std::io::stdout().flush().ok();

                let mut run_results = Vec::new();
                for _ in 0..args.runs {
                    // Flush Redis DB
                    if let Ok(mut client) = RespClient::connect(&args.host, args.redis_port) {
                        let _ = client.flushdb();
                    }

                    // Pre-populate for read-heavy
                    if matches!(workload, WorkloadType::ReadHeavy) {
                        if let Ok(mut client) = RespClient::connect(&args.host, args.redis_port) {
                            let value = "x".repeat(args.value_size);
                            let populate_count = args.key_space.min(5000);
                            for i in 0..populate_count {
                                let _ = client.set(&format!("bench:key:{}", i), &value);
                            }
                        }
                    }

                    let result = run_single_workload(
                        &args.host,
                        args.redis_port,
                        conc,
                        args.requests,
                        args.key_space,
                        args.value_size,
                        *workload,
                        "Redis",
                    );
                    run_results.push(result);
                }

                let agg = aggregate_runs(run_results);
                let cv = if agg.ops_per_sec_mean > 0.0 {
                    agg.ops_per_sec_stddev / agg.ops_per_sec_mean * 100.0
                } else {
                    0.0
                };
                println!(
                    "{:>7.0} ± {:<5.0} ops/sec (CV={:.1}%) | p50={:.0}±{:.0}µs  p99={:.0}±{:.0}µs",
                    agg.ops_per_sec_mean, agg.ops_per_sec_stddev, cv,
                    agg.p50_us_mean, agg.p50_us_stddev,
                    agg.p99_us_mean, agg.p99_us_stddev,
                );

                all_results.push(agg);
            }
        }
    }

    // ── Output results ──────────────────────────────────────────────────────
    write_results(&args.output_dir, args.runs, all_results, memory_samples);

    // ── Metrics strategy comparison (if requested) ──────────────────────────
    if args.metrics_strategy != "none" {
        println!("\n━━━ Metrics Strategy Comparison ━━━");
        println!("Current server strategy: {}", args.metrics_strategy);
        println!("  (restart server with RUSTREDIS_METRICS_STRATEGY=<strategy> for each comparison)");
        println!();

        // Run a mixed workload at a single concurrency for the current strategy
        let test_concurrency = 100;
        let test_requests = args.requests;

        print!(
            "  Strategy: {:20} | {} clients × {} ops × {} runs ... ",
            args.metrics_strategy, test_concurrency, test_requests, args.runs
        );
        std::io::stdout().flush().ok();

        let mut run_results = Vec::new();
        for _ in 0..args.runs {
            if let Ok(mut client) = RespClient::connect(&args.host, args.port) {
                let _ = client.flushdb();
            }

            let result = run_single_workload(
                &args.host,
                args.port,
                test_concurrency,
                test_requests,
                args.key_space,
                args.value_size,
                WorkloadType::Mixed,
                &format!("RustRedis-{}", args.metrics_strategy),
            );
            run_results.push(result);
        }

        let agg = aggregate_runs(run_results);
        println!(
            "{:>7.0} ± {:<5.0} ops/sec | p99={:.0}±{:.0}µs",
            agg.ops_per_sec_mean, agg.ops_per_sec_stddev,
            agg.p99_us_mean, agg.p99_us_stddev,
        );

        // Fetch CMDSTAT from the server
        if let Ok(mut client) = RespClient::connect(&args.host, args.port) {
            if let Ok(stats) = client.cmdstat() {
                println!("\n  Per-command stats (from CMDSTAT):");
                for line in stats.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('$') && !line.starts_with('*') {
                        println!("    {}", line);
                    }
                }
            }
        }

        println!();
        println!("  ┌─── Metrics Strategy Comparison Guide ────────────────────┐");
        println!("  │ To compare strategies, run this benchmark 4 times:      │");
        println!("  │                                                         │");
        println!("  │ 1. RUSTREDIS_METRICS_STRATEGY=disabled                  │");
        println!("  │    cargo run --release --bin server                      │");
        println!("  │    cargo run --release -- --metrics-strategy disabled    │");
        println!("  │                                                         │");
        println!("  │ 2. RUSTREDIS_METRICS_STRATEGY=global_mutex              │");
        println!("  │    (repeat server + benchmark)                          │");
        println!("  │                                                         │");
        println!("  │ 3. RUSTREDIS_METRICS_STRATEGY=sharded                   │");
        println!("  │    (repeat server + benchmark)                          │");
        println!("  │                                                         │");
        println!("  │ 4. RUSTREDIS_METRICS_STRATEGY=thread_local              │");
        println!("  │    (repeat server + benchmark)                          │");
        println!("  │                                                         │");
        println!("  │ Compare ops/sec and p99 across runs to measure overhead │");
        println!("  └─────────────────────────────────────────────────────────┘");
    }
}

fn write_results(
    output_dir: &str,
    runs_per_config: usize,
    results: Vec<AggregatedResult>,
    memory_samples: Vec<MemorySample>,
) {
    let suite = BenchmarkSuite {
        timestamp: chrono_now(),
        runs_per_config,
        results: results.clone(),
        memory_samples: memory_samples.clone(),
    };

    // Write JSON
    let json = serde_json::to_string_pretty(&suite).unwrap();
    let json_path = format!("{}/benchmark_results.json", output_dir);
    std::fs::write(&json_path, &json).expect("Failed to write results JSON");
    println!("\n✓ Results saved to {}", json_path);

    // Print summary table
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              BENCHMARK SUMMARY (mean ± stddev, n={})                               ║", runs_per_config);
    println!("╠═══════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!(
        "║ {:8} │ {:30} │ {:>18} │ {:>14} │ {:>14} ║",
        "Target", "Workload", "ops/sec", "p50(µs)", "p99(µs)"
    );
    println!("╟──────────┼────────────────────────────────┼────────────────────┼────────────────┼────────────────╢");

    for r in &results {
        println!(
            "║ {:8} │ {:30} │ {:>8.0} ± {:<6.0} │ {:>6.0} ± {:<5.0} │ {:>6.0} ± {:<5.0} ║",
            format!("{}@c{}", r.target, r.concurrency),
            r.name,
            r.ops_per_sec_mean, r.ops_per_sec_stddev,
            r.p50_us_mean, r.p50_us_stddev,
            r.p99_us_mean, r.p99_us_stddev,
        );
    }
    println!("╚═══════════════════════════════════════════════════════════════════════════════════════════════════════╝");

    // Print comparison if both Redis and RustRedis results exist
    let rust_results: Vec<_> = results.iter().filter(|r| r.target == "RustRedis").collect();
    let redis_results: Vec<_> = results.iter().filter(|r| r.target == "Redis").collect();

    if !redis_results.is_empty() {
        println!("\n┌─── Performance Comparison (mean values) ──────────────┐");
        for rr in &rust_results {
            if let Some(redis) = redis_results
                .iter()
                .find(|r| r.concurrency == rr.concurrency && r.name == rr.name)
            {
                let throughput_diff = (rr.ops_per_sec_mean / redis.ops_per_sec_mean - 1.0) * 100.0;
                let latency_diff = (rr.p99_us_mean / redis.p99_us_mean - 1.0) * 100.0;
                println!(
                    "│ {} c={}: throughput {:+.1}%, p99 latency {:+.1}%",
                    rr.name, rr.concurrency, throughput_diff, latency_diff
                );
            }
        }
        println!("└────────────────────────────────────────────────────────┘");
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", secs)
}
