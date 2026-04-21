//! Scalable concurrent metrics collection for observability strategy experiments.
//!
//! This module provides interchangeable collectors to measure how telemetry design
//! impacts server performance under load.

use dashmap::DashMap;
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const FLUSH_TRIGGER_EVERY_RECORDS: u64 = 1000;
const HDR_LOWEST_TRACKABLE_US: u64 = 1;
const HDR_HIGHEST_TRACKABLE_US: u64 = 3_600_000_000;
const HDR_SIGNIFICANT_FIGURES: u8 = 3;
const CMDSTAT_MAX_LINES: usize = 500;

// =============================================================================
// Configuration
// =============================================================================

/// Strategy for per-command/per-key metrics collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsStrategy {
    /// No telemetry in the request hot path.
    Disabled,
    /// Single global `Mutex<HashMap>`.
    GlobalMutex,
    /// DashMap keyed by command name (effectively 2-key collapse for GET/SET).
    Sharded2Key,
    /// DashMap keyed by full logical key (large-key-space sharding).
    ShardedN,
    /// Thread-local counters with periodic merge.
    ThreadLocalBatched,
    /// Per-thread HdrHistogram with periodic merge.
    HdrHistogram,
}

impl MetricsStrategy {
    /// Parse from string (case-insensitive). Falls back to `Sharded2Key`.
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "disabled" | "none" | "off" => MetricsStrategy::Disabled,
            "global_mutex" | "globalmutex" | "mutex" => MetricsStrategy::GlobalMutex,
            "sharded" | "dashmap" | "sharded_2key" | "sharded2key" => {
                MetricsStrategy::Sharded2Key
            }
            "sharded_n" | "shardedn" | "sharded_full" | "sharded_full_key" => {
                MetricsStrategy::ShardedN
            }
            "thread_local" | "threadlocal" | "thread_local_batched" | "tls" => {
                MetricsStrategy::ThreadLocalBatched
            }
            "hdr" | "hdrhistogram" | "hdr_histogram" | "histogram" => {
                MetricsStrategy::HdrHistogram
            }
            _ => MetricsStrategy::Sharded2Key,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MetricsStrategy::Disabled => "disabled",
            MetricsStrategy::GlobalMutex => "global_mutex",
            MetricsStrategy::Sharded2Key => "sharded_2key",
            MetricsStrategy::ShardedN => "sharded_n",
            MetricsStrategy::ThreadLocalBatched => "thread_local",
            MetricsStrategy::HdrHistogram => "hdr_histogram",
        }
    }
}

// =============================================================================
// Per-command stat
// =============================================================================

/// Accumulated statistics for a single metric key.
#[derive(Debug, Clone)]
pub struct CommandStat {
    pub calls: u64,
    pub total_time_us: u64,
    pub min_time_us: u64,
    pub max_time_us: u64,
}

impl CommandStat {
    fn new() -> Self {
        CommandStat {
            calls: 0,
            total_time_us: 0,
            min_time_us: u64::MAX,
            max_time_us: 0,
        }
    }

    fn record(&mut self, duration_us: u64) {
        self.calls += 1;
        self.total_time_us += duration_us;
        if duration_us < self.min_time_us {
            self.min_time_us = duration_us;
        }
        if duration_us > self.max_time_us {
            self.max_time_us = duration_us;
        }
    }

    fn merge(&mut self, other: &CommandStat) {
        self.calls += other.calls;
        self.total_time_us += other.total_time_us;
        if other.min_time_us < self.min_time_us {
            self.min_time_us = other.min_time_us;
        }
        if other.max_time_us > self.max_time_us {
            self.max_time_us = other.max_time_us;
        }
    }

    /// Average execution time in microseconds. Returns 0 if no calls.
    pub fn avg_time_us(&self) -> f64 {
        if self.calls > 0 {
            self.total_time_us as f64 / self.calls as f64
        } else {
            0.0
        }
    }

    /// Display-safe min value (0 when no samples exist).
    pub fn display_min(&self) -> u64 {
        if self.calls == 0 {
            0
        } else {
            self.min_time_us
        }
    }
}

// =============================================================================
// Strategy A: Global Mutex
// =============================================================================

struct GlobalMutexCollector {
    data: Mutex<HashMap<&'static str, CommandStat>>,
    lock_wait_us: AtomicU64,
}

impl GlobalMutexCollector {
    fn new() -> Self {
        GlobalMutexCollector {
            data: Mutex::new(HashMap::new()),
            lock_wait_us: AtomicU64::new(0),
        }
    }

    fn record(&self, cmd_name: &'static str, duration_us: u64) {
        let wait_start = Instant::now();
        let mut map = self.data.lock().unwrap();
        let wait_us = wait_start.elapsed().as_micros() as u64;
        self.lock_wait_us.fetch_add(wait_us, Ordering::Relaxed);

        map.entry(cmd_name)
            .or_insert_with(CommandStat::new)
            .record(duration_us);
    }

    fn snapshot(&self) -> Vec<(&'static str, CommandStat)> {
        let map = self.data.lock().unwrap();
        map.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    fn lock_wait_us(&self) -> u64 {
        self.lock_wait_us.load(Ordering::Relaxed)
    }
}

// =============================================================================
// Strategy B: Sharded-2key (DashMap keyed by command)
// =============================================================================

struct Sharded2KeyCollector {
    data: DashMap<&'static str, CommandStat>,
}

impl Sharded2KeyCollector {
    fn new() -> Self {
        Sharded2KeyCollector {
            data: DashMap::new(),
        }
    }

    fn record(&self, cmd_name: &'static str, duration_us: u64) {
        self.data
            .entry(cmd_name)
            .and_modify(|stat| stat.record(duration_us))
            .or_insert_with(|| {
                let mut s = CommandStat::new();
                s.record(duration_us);
                s
            });
    }

    fn snapshot(&self) -> Vec<(&'static str, CommandStat)> {
        self.data
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }
}

// =============================================================================
// Strategy C: Sharded-N (DashMap keyed by full logical key)
// =============================================================================

struct ShardedNCollector {
    data: DashMap<String, CommandStat>,
}

impl ShardedNCollector {
    fn new() -> Self {
        ShardedNCollector {
            data: DashMap::new(),
        }
    }

    fn record(&self, metric_key: &str, duration_us: u64) {
        self.data
            .entry(metric_key.to_string())
            .and_modify(|stat| stat.record(duration_us))
            .or_insert_with(|| {
                let mut s = CommandStat::new();
                s.record(duration_us);
                s
            });
    }

    fn snapshot(&self) -> Vec<(String, CommandStat)> {
        self.data
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

// =============================================================================
// Strategy D: Thread-Local Batched
// =============================================================================

thread_local! {
    static TLS_STATS: std::cell::RefCell<HashMap<&'static str, CommandStat>> =
        std::cell::RefCell::new(HashMap::new());
}

pub struct ThreadLocalBatchedCollector {
    global_snapshot: Mutex<HashMap<&'static str, CommandStat>>,
    pending_batches: Mutex<Vec<HashMap<&'static str, CommandStat>>>,
    records_since_flush: AtomicU64,
}

impl ThreadLocalBatchedCollector {
    fn new() -> Self {
        ThreadLocalBatchedCollector {
            global_snapshot: Mutex::new(HashMap::new()),
            pending_batches: Mutex::new(Vec::new()),
            records_since_flush: AtomicU64::new(0),
        }
    }

    fn record(&self, cmd_name: &'static str, duration_us: u64) {
        TLS_STATS.with(|tls| {
            let mut map = tls.borrow_mut();
            map.entry(cmd_name)
                .or_insert_with(CommandStat::new)
                .record(duration_us);
        });

        let count = self.records_since_flush.fetch_add(1, Ordering::Relaxed);
        if count % FLUSH_TRIGGER_EVERY_RECORDS == FLUSH_TRIGGER_EVERY_RECORDS - 1 {
            self.push_local_batch();
        }
    }

    fn push_local_batch(&self) {
        TLS_STATS.with(|tls| {
            let mut local = tls.borrow_mut();
            if !local.is_empty() {
                let batch = std::mem::take(&mut *local);
                if let Ok(mut pending) = self.pending_batches.lock() {
                    pending.push(batch);
                }
            }
        });
    }

    fn flush(&self) {
        let batches: Vec<HashMap<&'static str, CommandStat>> = {
            let mut pending = self.pending_batches.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        if batches.is_empty() {
            return;
        }

        let mut snapshot = self.global_snapshot.lock().unwrap();
        for batch in batches {
            for (cmd, stat) in batch {
                snapshot
                    .entry(cmd)
                    .or_insert_with(CommandStat::new)
                    .merge(&stat);
            }
        }

        self.records_since_flush.store(0, Ordering::Relaxed);
    }

    fn snapshot(&self) -> Vec<(&'static str, CommandStat)> {
        self.flush();
        self.push_local_batch();
        self.flush();

        let snapshot = self.global_snapshot.lock().unwrap();
        snapshot.iter().map(|(k, v)| (*k, v.clone())).collect()
    }
}

// =============================================================================
// Strategy E: HdrHistogram (per-thread + periodic merge)
// =============================================================================

#[derive(Clone)]
struct HdrThreadStat {
    calls: u64,
    total_time_us: u64,
    min_time_us: u64,
    max_time_us: u64,
    histogram: Histogram<u64>,
}

impl HdrThreadStat {
    fn new() -> Self {
        let histogram = Histogram::<u64>::new_with_bounds(
            HDR_LOWEST_TRACKABLE_US,
            HDR_HIGHEST_TRACKABLE_US,
            HDR_SIGNIFICANT_FIGURES,
        )
        .expect("failed to initialize HdrHistogram");

        HdrThreadStat {
            calls: 0,
            total_time_us: 0,
            min_time_us: u64::MAX,
            max_time_us: 0,
            histogram,
        }
    }

    fn record(&mut self, duration_us: u64) {
        self.calls += 1;
        self.total_time_us += duration_us;
        if duration_us < self.min_time_us {
            self.min_time_us = duration_us;
        }
        if duration_us > self.max_time_us {
            self.max_time_us = duration_us;
        }

        let clamped = duration_us.clamp(HDR_LOWEST_TRACKABLE_US, HDR_HIGHEST_TRACKABLE_US);
        let _ = self.histogram.record(clamped);
    }

    fn merge(&mut self, other: &HdrThreadStat) {
        self.calls += other.calls;
        self.total_time_us += other.total_time_us;
        if other.min_time_us < self.min_time_us {
            self.min_time_us = other.min_time_us;
        }
        if other.max_time_us > self.max_time_us {
            self.max_time_us = other.max_time_us;
        }
        let _ = self.histogram.add(&other.histogram);
    }

    fn as_command_stat(&self) -> CommandStat {
        CommandStat {
            calls: self.calls,
            total_time_us: self.total_time_us,
            min_time_us: if self.calls == 0 { 0 } else { self.min_time_us },
            max_time_us: self.max_time_us,
        }
    }
}

thread_local! {
    static TLS_HDR_STATS: std::cell::RefCell<HashMap<String, HdrThreadStat>> =
        std::cell::RefCell::new(HashMap::new());
}

pub struct HdrHistogramCollector {
    global_snapshot: Mutex<HashMap<String, HdrThreadStat>>,
    pending_batches: Mutex<Vec<HashMap<String, HdrThreadStat>>>,
    records_since_flush: AtomicU64,
}

impl HdrHistogramCollector {
    fn new() -> Self {
        HdrHistogramCollector {
            global_snapshot: Mutex::new(HashMap::new()),
            pending_batches: Mutex::new(Vec::new()),
            records_since_flush: AtomicU64::new(0),
        }
    }

    fn record(&self, metric_key: &str, duration_us: u64) {
        TLS_HDR_STATS.with(|tls| {
            let mut map = tls.borrow_mut();
            map.entry(metric_key.to_string())
                .or_insert_with(HdrThreadStat::new)
                .record(duration_us);
        });

        let count = self.records_since_flush.fetch_add(1, Ordering::Relaxed);
        if count % FLUSH_TRIGGER_EVERY_RECORDS == FLUSH_TRIGGER_EVERY_RECORDS - 1 {
            self.push_local_batch();
        }
    }

    fn push_local_batch(&self) {
        TLS_HDR_STATS.with(|tls| {
            let mut local = tls.borrow_mut();
            if !local.is_empty() {
                let batch = std::mem::take(&mut *local);
                if let Ok(mut pending) = self.pending_batches.lock() {
                    pending.push(batch);
                }
            }
        });
    }

    fn flush(&self) {
        let batches: Vec<HashMap<String, HdrThreadStat>> = {
            let mut pending = self.pending_batches.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        if batches.is_empty() {
            return;
        }

        let mut snapshot = self.global_snapshot.lock().unwrap();
        for batch in batches {
            for (metric_key, stat) in batch {
                snapshot
                    .entry(metric_key)
                    .or_insert_with(HdrThreadStat::new)
                    .merge(&stat);
            }
        }

        self.records_since_flush.store(0, Ordering::Relaxed);
    }

    fn snapshot(&self) -> Vec<(String, CommandStat)> {
        self.flush();
        self.push_local_batch();
        self.flush();

        let snapshot = self.global_snapshot.lock().unwrap();
        snapshot
            .iter()
            .map(|(key, stat)| (key.clone(), stat.as_command_stat()))
            .collect()
    }
}

// =============================================================================
// Unified Public API
// =============================================================================

pub struct CommandMetricsCollector {
    strategy: MetricsStrategy,
    global_mutex: Option<GlobalMutexCollector>,
    sharded_2key: Option<Sharded2KeyCollector>,
    sharded_n: Option<ShardedNCollector>,
    thread_local: Option<Arc<ThreadLocalBatchedCollector>>,
    hdr_histogram: Option<Arc<HdrHistogramCollector>>,
}

pub type SharedCommandMetrics = Arc<CommandMetricsCollector>;

impl CommandMetricsCollector {
    pub fn new(strategy: MetricsStrategy) -> SharedCommandMetrics {
        let (global_mutex, sharded_2key, sharded_n, thread_local, hdr_histogram) = match strategy {
            MetricsStrategy::Disabled => (None, None, None, None, None),
            MetricsStrategy::GlobalMutex => (Some(GlobalMutexCollector::new()), None, None, None, None),
            MetricsStrategy::Sharded2Key => (None, Some(Sharded2KeyCollector::new()), None, None, None),
            MetricsStrategy::ShardedN => (None, None, Some(ShardedNCollector::new()), None, None),
            MetricsStrategy::ThreadLocalBatched => {
                (None, None, None, Some(Arc::new(ThreadLocalBatchedCollector::new())), None)
            }
            MetricsStrategy::HdrHistogram => {
                (None, None, None, None, Some(Arc::new(HdrHistogramCollector::new())))
            }
        };

        Arc::new(CommandMetricsCollector {
            strategy,
            global_mutex,
            sharded_2key,
            sharded_n,
            thread_local,
            hdr_histogram,
        })
    }

    /// Record a command execution.
    ///
    /// `cmd_name` is the canonical command string (GET/SET/etc).
    /// `key_hint` is the logical key used for key-space sharding strategies.
    #[inline]
    pub fn record(&self, cmd_name: &'static str, key_hint: Option<&str>, duration_us: u64) {
        match self.strategy {
            MetricsStrategy::Disabled => {}
            MetricsStrategy::GlobalMutex => {
                if let Some(ref collector) = self.global_mutex {
                    collector.record(cmd_name, duration_us);
                }
            }
            MetricsStrategy::Sharded2Key => {
                if let Some(ref collector) = self.sharded_2key {
                    collector.record(cmd_name, duration_us);
                }
            }
            MetricsStrategy::ShardedN => {
                if let Some(ref collector) = self.sharded_n {
                    collector.record(key_hint.unwrap_or(cmd_name), duration_us);
                }
            }
            MetricsStrategy::ThreadLocalBatched => {
                if let Some(ref collector) = self.thread_local {
                    collector.record(cmd_name, duration_us);
                }
            }
            MetricsStrategy::HdrHistogram => {
                if let Some(ref collector) = self.hdr_histogram {
                    collector.record(cmd_name, duration_us);
                }
            }
        }
    }

    /// Get a snapshot of all metric entries sorted alphabetically by key.
    pub fn snapshot(&self) -> Vec<(String, CommandStat)> {
        let mut stats = match self.strategy {
            MetricsStrategy::Disabled => Vec::new(),
            MetricsStrategy::GlobalMutex => self
                .global_mutex
                .as_ref()
                .map(|c| {
                    c.snapshot()
                        .into_iter()
                        .map(|(name, stat)| (name.to_string(), stat))
                        .collect()
                })
                .unwrap_or_default(),
            MetricsStrategy::Sharded2Key => self
                .sharded_2key
                .as_ref()
                .map(|c| {
                    c.snapshot()
                        .into_iter()
                        .map(|(name, stat)| (name.to_string(), stat))
                        .collect()
                })
                .unwrap_or_default(),
            MetricsStrategy::ShardedN => self
                .sharded_n
                .as_ref()
                .map(|c| c.snapshot())
                .unwrap_or_default(),
            MetricsStrategy::ThreadLocalBatched => self
                .thread_local
                .as_ref()
                .map(|c| {
                    c.snapshot()
                        .into_iter()
                        .map(|(name, stat)| (name.to_string(), stat))
                        .collect()
                })
                .unwrap_or_default(),
            MetricsStrategy::HdrHistogram => self
                .hdr_histogram
                .as_ref()
                .map(|c| c.snapshot())
                .unwrap_or_default(),
        };

        stats.sort_by(|a, b| a.0.cmp(&b.0));
        stats
    }

    pub fn strategy_name(&self) -> &'static str {
        self.strategy.name()
    }

    pub fn lock_wait_us(&self) -> u64 {
        match self.strategy {
            MetricsStrategy::GlobalMutex => {
                self.global_mutex.as_ref().map(|c| c.lock_wait_us()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    pub fn thread_local_collector(&self) -> Option<Arc<ThreadLocalBatchedCollector>> {
        self.thread_local.clone()
    }

    pub fn hdr_histogram_collector(&self) -> Option<Arc<HdrHistogramCollector>> {
        self.hdr_histogram.clone()
    }

    pub fn format_cmdstat(&self) -> String {
        let stats = self.snapshot();
        let mut output = format!("# CommandStats (strategy: {})\r\n", self.strategy_name());

        if stats.is_empty() {
            output.push_str("(no commands recorded)\r\n");
            return output;
        }

        for (metric_key, stat) in stats.iter().take(CMDSTAT_MAX_LINES) {
            output.push_str(&format!(
                "cmdstat_{}:calls={},total_time_us={},avg_time_us={:.2},min_time_us={},max_time_us={}\r\n",
                sanitize_metric_key(metric_key),
                stat.calls,
                stat.total_time_us,
                stat.avg_time_us(),
                stat.display_min(),
                stat.max_time_us,
            ));
        }

        if stats.len() > CMDSTAT_MAX_LINES {
            output.push_str(&format!(
                "cmdstat_truncated_entries:{}\r\n",
                stats.len() - CMDSTAT_MAX_LINES
            ));
        }

        if self.strategy == MetricsStrategy::GlobalMutex {
            output.push_str(&format!(
                "\r\n# Contention\r\ncmdstat_lock_wait_us:{}\r\n",
                self.lock_wait_us()
            ));
        }

        output
    }
}

fn sanitize_metric_key(metric_key: &str) -> String {
    metric_key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub fn start_thread_local_flush_task(collector: Arc<ThreadLocalBatchedCollector>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            collector.flush();
        }
    });
}

pub fn start_hdr_flush_task(collector: Arc<HdrHistogramCollector>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            collector.flush();
        }
    });
}

/// Backward-compatible alias for existing server startup code.
pub fn start_flush_task(collector: Arc<ThreadLocalBatchedCollector>) {
    start_thread_local_flush_task(collector);
}
