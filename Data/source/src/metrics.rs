use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Global metrics for the RustRedis server.
///
/// All counters use relaxed atomic ordering for maximum performance.
/// This is acceptable because we only need approximate values for
/// observability — strict ordering is unnecessary for monotonic counters.
#[derive(Debug)]
pub struct Metrics {
    /// Total number of commands processed since server start
    total_commands: AtomicU64,

    /// Currently active client connections
    active_connections: AtomicU64,

    /// Cumulative command execution time in microseconds
    total_command_duration_us: AtomicU64,

    /// Cumulative AOF write time in microseconds
    total_aof_write_time_us: AtomicU64,

    /// Cumulative lock wait time in microseconds (Mutex acquisition)
    total_lock_wait_time_us: AtomicU64,

    /// Server start time for uptime calculation
    start_time: Instant,
}

/// Shared metrics handle — cheap to clone via Arc
pub type SharedMetrics = Arc<Metrics>;

impl Metrics {
    /// Create a new metrics instance
    pub fn new() -> SharedMetrics {
        Arc::new(Metrics {
            total_commands: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            total_command_duration_us: AtomicU64::new(0),
            total_aof_write_time_us: AtomicU64::new(0),
            total_lock_wait_time_us: AtomicU64::new(0),
            start_time: Instant::now(),
        })
    }

    // ===== Increment Operations =====

    pub fn increment_commands(&self) {
        self.total_commands.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn add_command_duration_us(&self, us: u64) {
        self.total_command_duration_us
            .fetch_add(us, Ordering::Relaxed);
    }

    pub fn add_aof_write_time_us(&self, us: u64) {
        self.total_aof_write_time_us
            .fetch_add(us, Ordering::Relaxed);
    }

    pub fn add_lock_wait_time_us(&self, us: u64) {
        self.total_lock_wait_time_us
            .fetch_add(us, Ordering::Relaxed);
    }

    // ===== Read Operations =====

    pub fn total_commands(&self) -> u64 {
        self.total_commands.load(Ordering::Relaxed)
    }

    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn total_command_duration_us(&self) -> u64 {
        self.total_command_duration_us.load(Ordering::Relaxed)
    }

    pub fn total_aof_write_time_us(&self) -> u64 {
        self.total_aof_write_time_us.load(Ordering::Relaxed)
    }

    pub fn total_lock_wait_time_us(&self) -> u64 {
        self.total_lock_wait_time_us.load(Ordering::Relaxed)
    }

    // ===== Computed Metrics =====

    /// Uptime in seconds
    pub fn uptime_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Average operations per second over the server lifetime
    pub fn ops_per_second(&self) -> f64 {
        let elapsed = self.uptime_secs();
        if elapsed > 0.0 {
            self.total_commands() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Average command duration in microseconds
    pub fn avg_command_duration_us(&self) -> f64 {
        let total = self.total_commands();
        if total > 0 {
            self.total_command_duration_us() as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Format all metrics as a human-readable multi-line string (for STATS command)
    pub fn format_stats(&self) -> String {
        format!(
            "# Server\r\n\
             uptime_seconds:{:.2}\r\n\
             \r\n\
             # Clients\r\n\
             connected_clients:{}\r\n\
             \r\n\
             # Stats\r\n\
             total_commands_processed:{}\r\n\
             instantaneous_ops_per_sec:{:.2}\r\n\
             avg_command_duration_us:{:.2}\r\n\
             \r\n\
             # Persistence\r\n\
             total_aof_write_time_us:{}\r\n\
             \r\n\
             # Contention\r\n\
             total_lock_wait_time_us:{}\r\n",
            self.uptime_secs(),
            self.active_connections(),
            self.total_commands(),
            self.ops_per_second(),
            self.avg_command_duration_us(),
            self.total_aof_write_time_us(),
            self.total_lock_wait_time_us(),
        )
    }

    /// Read memory usage from /proc/self/statm (Linux only).
    /// Returns (virtual_bytes, rss_bytes) or None on failure.
    pub fn memory_usage() -> Option<(u64, u64)> {
        let contents = std::fs::read_to_string("/proc/self/statm").ok()?;
        let mut parts = contents.split_whitespace();
        let vsize_pages: u64 = parts.next()?.parse().ok()?;
        let rss_pages: u64 = parts.next()?.parse().ok()?;
        let page_size = 4096u64; // Standard Linux page size
        Some((vsize_pages * page_size, rss_pages * page_size))
    }
}

impl Default for Metrics {
    fn default() -> Self {
        // This is only used for the inner type; prefer Metrics::new() which returns Arc
        Metrics {
            total_commands: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            total_command_duration_us: AtomicU64::new(0),
            total_aof_write_time_us: AtomicU64::new(0),
            total_lock_wait_time_us: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}
