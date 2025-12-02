use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Shared database handle
///
/// The database is a simple key-value store backed by a HashMap.
/// It's wrapped in Arc<Mutex<>> for thread-safe shared access across
/// async tasks.
#[derive(Clone)]
pub struct Db {
    /// The shared state containing the actual HashMap
    shared: Arc<Mutex<DbState>>,
}

/// Database entry with optional expiration
struct Entry {
    /// The value stored
    value: Bytes,

    /// Optional expiration time
    expires_at: Option<Instant>,
}

/// The actual database state
struct DbState {
    /// Key-value storage
    entries: HashMap<String, Entry>,
}

impl Db {
    /// Create a new database instance
    pub fn new() -> Db {
        Db {
            shared: Arc::new(Mutex::new(DbState {
                entries: HashMap::new(),
            })),
        }
    }

    /// Read an entry from the database
    ///
    /// Returns `Some(value)` if the key exists and has not expired
    /// Returns `None` if the key doesn't exist or has expired
    /// Automatically removes expired entries
    pub fn read_entry(&self, key: &str) -> Option<Bytes> {
        let mut state = self.shared.lock().unwrap();

        // Check if entry exists
        let entry = state.entries.get(key)?;

        // Check if entry has expired
        if let Some(expires_at) = entry.expires_at {
            if Instant::now() >= expires_at {
                // Entry has expired, remove it
                state.entries.remove(key);
                return None;
            }
        }

        // Entry exists and hasn't expired
        Some(entry.value.clone())
    }

    /// Write an entry to the database without expiration
    ///
    /// If the key already exists, its value will be overwritten
    pub fn write_entry(&self, key: String, value: Bytes) {
        self.write_entry_with_expiration(key, value, None);
    }

    /// Write an entry to the database with optional expiration
    ///
    /// If expires_at is Some, the entry will expire at the given time
    pub fn write_entry_with_expiration(
        &self,
        key: String,
        value: Bytes,
        expires_at: Option<Instant>,
    ) {
        let mut state = self.shared.lock().unwrap();
        state.entries.insert(key, Entry { value, expires_at });
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
