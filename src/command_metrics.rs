//! Scalable Concurrent Metrics Collection System
//!
//! Inspired by PostgreSQL's `pg_stat_statements`, this module tracks per-command
//! telemetry (calls, total/min/max execution time) using three interchangeable
//! concurrency strategies to study contention characteristics under high load.
//!
//! # Strategies
//!
//! - **GlobalMutex**: Single `Mutex<HashMap>` — baseline for contention measurement
//! - **Sharded**: `DashMap` — sharded locking, parallel updates for different commands
//! - **ThreadLocalBatched**: Thread-local counters flushed periodically — zero contention on hot path

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// =============================================================================
// Configuration
// =============================================================================

/// Strategy for per-command metrics collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsStrategy {
    /// No per-command metrics — zero overhead baseline
    Disabled,
    /// Single global `Mutex<HashMap>` — maximum contention
    GlobalMutex,
    /// DashMap sharded lock — reduced contention via partitioning
    Sharded,
    /// Thread-local counters with periodic flush — near-zero hot-path contention
    ThreadLocalBatched,
}

impl MetricsStrategy {
    /// Parse from string (case-insensitive). Returns `Sharded` as default.
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "disabled" | "none" | "off" => MetricsStrategy::Disabled,
            "global_mutex" | "globalmutex" | "mutex" => MetricsStrategy::GlobalMutex,
            "sharded" | "dashmap" => MetricsStrategy::Sharded,
            "thread_local" | "threadlocal" | "thread_local_batched" | "tls" => {
                MetricsStrategy::ThreadLocalBatched
            }
            _ => MetricsStrategy::Sharded,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MetricsStrategy::Disabled => "disabled",
            MetricsStrategy::GlobalMutex => "global_mutex",
            MetricsStrategy::Sharded => "sharded",
            MetricsStrategy::ThreadLocalBatched => "thread_local_batched",
        }
    }
}

// =============================================================================
// Per-command stat
// =============================================================================

/// Accumulated statistics for a single command type.
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

    /// Display-safe min_time_us (shows 0 instead of u64::MAX when no calls).
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
    /// Cumulative time threads spent waiting for this lock (microseconds).
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

        map.entry(cmd_name).or_insert_with(CommandStat::new).record(duration_us);
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
// Strategy B: Sharded (DashMap)
// =============================================================================

struct ShardedCollector {
    data: DashMap<&'static str, CommandStat>,
}

impl ShardedCollector {
    fn new() -> Self {
        ShardedCollector {
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
        self.data.iter().map(|entry| (*entry.key(), entry.value().clone())).collect()
    }
}

// =============================================================================
// Strategy C: Thread-Local Batched
// =============================================================================

// Thread-local storage for per-thread command stats.
// When flushed, the local map is drained into the global snapshot.
thread_local! {
    static TLS_STATS: std::cell::RefCell<HashMap<&'static str, CommandStat>> =
        std::cell::RefCell::new(HashMap::new());
}

pub struct ThreadLocalBatchedCollector {
    /// Global snapshot aggregated from all thread-local flushes.
    global_snapshot: Mutex<HashMap<&'static str, CommandStat>>,
    /// Pending batches pushed by worker threads, drained by the flush task.
    pending_batches: Mutex<Vec<HashMap<&'static str, CommandStat>>>,
    /// Total number of records since last flush (approximate, for triggering).
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
        // Record into thread-local storage — zero synchronization
        TLS_STATS.with(|tls| {
            let mut map = tls.borrow_mut();
            map.entry(cmd_name)
                .or_insert_with(CommandStat::new)
                .record(duration_us);
        });

        let count = self.records_since_flush.fetch_add(1, Ordering::Relaxed);

        // Every 1000 records, push the local batch for aggregation
        if count % 1000 == 999 {
            self.push_local_batch();
        }
    }

    /// Push the current thread's local stats into the pending batches queue.
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

    /// Flush all pending batches into the global snapshot.
    /// Called periodically by the background flush task.
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
        // First, flush any pending batches to ensure freshness
        self.flush();

        // Also push current thread's local batch (for CMDSTAT caller's thread)
        self.push_local_batch();
        self.flush();

        let snapshot = self.global_snapshot.lock().unwrap();
        snapshot.iter().map(|(k, v)| (*k, v.clone())).collect()
    }
}

// =============================================================================
// Unified Public API
// =============================================================================

/// The unified command metrics collector. Wraps one of the three strategy backends.
pub struct CommandMetricsCollector {
    strategy: MetricsStrategy,
    global_mutex: Option<GlobalMutexCollector>,
    sharded: Option<ShardedCollector>,
    thread_local: Option<Arc<ThreadLocalBatchedCollector>>,
}

/// Shared handle for the command metrics collector.
pub type SharedCommandMetrics = Arc<CommandMetricsCollector>;

impl CommandMetricsCollector {
    /// Create a new collector with the given strategy.
    pub fn new(strategy: MetricsStrategy) -> SharedCommandMetrics {
        let (global_mutex, sharded, thread_local) = match strategy {
            MetricsStrategy::Disabled => (None, None, None),
            MetricsStrategy::GlobalMutex => (Some(GlobalMutexCollector::new()), None, None),
            MetricsStrategy::Sharded => (None, Some(ShardedCollector::new()), None),
            MetricsStrategy::ThreadLocalBatched => {
                (None, None, Some(Arc::new(ThreadLocalBatchedCollector::new())))
            }
        };

        Arc::new(CommandMetricsCollector {
            strategy,
            global_mutex,
            sharded,
            thread_local,
        })
    }

    /// Record a command execution on the hot path.
    ///
    /// This is designed to be as lightweight as possible for each strategy:
    /// - Disabled: no-op
    /// - GlobalMutex: acquires global lock
    /// - Sharded: acquires per-shard lock via DashMap
    /// - ThreadLocalBatched: writes to thread-local storage (no sync)
    #[inline]
    pub fn record(&self, cmd_name: &'static str, duration_us: u64) {
        match self.strategy {
            MetricsStrategy::Disabled => {}
            MetricsStrategy::GlobalMutex => {
                if let Some(ref collector) = self.global_mutex {
                    collector.record(cmd_name, duration_us);
                }
            }
            MetricsStrategy::Sharded => {
                if let Some(ref collector) = self.sharded {
                    collector.record(cmd_name, duration_us);
                }
            }
            MetricsStrategy::ThreadLocalBatched => {
                if let Some(ref collector) = self.thread_local {
                    collector.record(cmd_name, duration_us);
                }
            }
        }
    }

    /// Get a snapshot of all per-command statistics.
    /// Results are sorted alphabetically by command name.
    pub fn snapshot(&self) -> Vec<(&'static str, CommandStat)> {
        let mut stats = match self.strategy {
            MetricsStrategy::Disabled => Vec::new(),
            MetricsStrategy::GlobalMutex => {
                self.global_mutex.as_ref().map(|c| c.snapshot()).unwrap_or_default()
            }
            MetricsStrategy::Sharded => {
                self.sharded.as_ref().map(|c| c.snapshot()).unwrap_or_default()
            }
            MetricsStrategy::ThreadLocalBatched => {
                self.thread_local.as_ref().map(|c| c.snapshot()).unwrap_or_default()
            }
        };
        stats.sort_by_key(|(name, _)| *name);
        stats
    }

    /// Get the active strategy name.
    pub fn strategy_name(&self) -> &'static str {
        self.strategy.name()
    }

    /// Get the cumulative lock wait time (only meaningful for GlobalMutex).
    pub fn lock_wait_us(&self) -> u64 {
        match self.strategy {
            MetricsStrategy::GlobalMutex => {
                self.global_mutex.as_ref().map(|c| c.lock_wait_us()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Get a handle to the ThreadLocalBatched collector for background flushing.
    pub fn thread_local_collector(&self) -> Option<Arc<ThreadLocalBatchedCollector>> {
        self.thread_local.clone()
    }

    /// Format all per-command stats as a human-readable RESP-compatible string.
    pub fn format_cmdstat(&self) -> String {
        let stats = self.snapshot();
        let mut output = format!(
            "# CommandStats (strategy: {})\r\n",
            self.strategy_name()
        );

        if stats.is_empty() {
            output.push_str("(no commands recorded)\r\n");
            return output;
        }

        for (cmd, stat) in &stats {
            output.push_str(&format!(
                "cmdstat_{}:calls={},total_time_us={},avg_time_us={:.2},min_time_us={},max_time_us={}\r\n",
                cmd.to_lowercase(),
                stat.calls,
                stat.total_time_us,
                stat.avg_time_us(),
                stat.display_min(),
                stat.max_time_us,
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

/// Start the background flush task for ThreadLocalBatched strategy.
/// Should be called once during server initialization.
pub fn start_flush_task(collector: Arc<ThreadLocalBatchedCollector>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            collector.flush();
        }
    });
}
