use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

/// The actual database state
struct DbState {
    /// Key-value storage
    entries: HashMap<String, Bytes>,
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
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
