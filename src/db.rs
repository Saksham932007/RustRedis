use bytes::Bytes;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Value types supported by the database
#[derive(Clone, Debug)]
pub enum Value {
    String(Bytes),
    List(VecDeque<Bytes>),
    Set(HashSet<String>),
    Hash(HashMap<String, Bytes>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::Hash(_) => "hash",
        }
    }
}

/// Shared database handle
///
/// The database supports multiple data types: Strings, Lists, Sets, and Hashes.
/// It's wrapped in Arc<Mutex<>> for thread-safe shared access across async tasks.
#[derive(Clone)]
pub struct Db {
    /// The shared state containing the actual HashMap
    shared: Arc<Mutex<DbState>>,
}

/// Database entry with optional expiration
struct Entry {
    /// The value stored (can be String, List, Set, or Hash)
    value: Value,

    /// Optional expiration time
    expires_at: Option<Instant>,
}

/// The actual database state
struct DbState {
    /// Key-value storage supporting multiple data types
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

    /// Read a String value from the database
    ///
    /// Returns None if:
    /// - The key doesn't exist
    /// - The key has expired
    /// - The key contains a non-String value
    pub fn read_string(&self, key: &str) -> Option<Bytes> {
        let mut state = self.shared.lock().unwrap();

        // Check if the entry exists
        let entry = state.entries.get(key)?;

        // Check if the entry has expired
        if let Some(expires_at) = entry.expires_at {
            if Instant::now() >= expires_at {
                // Remove expired entry
                state.entries.remove(key);
                return None;
            }
        }

        // Return value only if it's a String type
        match &entry.value {
            Value::String(bytes) => Some(bytes.clone()),
            _ => None,
        }
    }

    /// Write a String value to the database with optional expiration
    pub fn write_string(&self, key: String, value: Bytes, expires_at: Option<Instant>) {
        let mut state = self.shared.lock().unwrap();

        let entry = Entry {
            value: Value::String(value),
            expires_at,
        };

        state.entries.insert(key, entry);
    }

    /// Get the type of a value
    pub fn get_type(&self, key: &str) -> Option<&'static str> {
        let state = self.shared.lock().unwrap();
        state.entries.get(key).map(|entry| entry.value.type_name())
    }

    /// Check if a key exists (and hasn't expired)
    pub fn exists(&self, key: &str) -> bool {
        let mut state = self.shared.lock().unwrap();

        if let Some(entry) = state.entries.get(key) {
            // Check if expired
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() >= expires_at {
                    state.entries.remove(key);
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Delete a key from the database
    pub fn delete(&self, key: &str) -> bool {
        let mut state = self.shared.lock().unwrap();
        state.entries.remove(key).is_some()
    }

    // ===== List Operations =====

    /// Push values to the left (head) of a list
    pub fn lpush(&self, key: String, values: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock().unwrap();

        let entry = state.entries.entry(key).or_insert_with(|| Entry {
            value: Value::List(VecDeque::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::List(list) => {
                for value in values.into_iter().rev() {
                    list.push_front(value);
                }
                list.len()
            }
            _ => 0, // Type error: key exists but isn't a list
        }
    }

    /// Push values to the right (tail) of a list
    pub fn rpush(&self, key: String, values: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock().unwrap();

        let entry = state.entries.entry(key).or_insert_with(|| Entry {
            value: Value::List(VecDeque::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::List(list) => {
                for value in values {
                    list.push_back(value);
                }
                list.len()
            }
            _ => 0,
        }
    }

    /// Pop a value from the left (head) of a list
    pub fn lpop(&self, key: &str) -> Option<Bytes> {
        let mut state = self.shared.lock().unwrap();

        state.entries.get_mut(key).and_then(|entry| {
            match &mut entry.value {
                Value::List(list) => list.pop_front(),
                _ => None,
            }
        })
    }

    /// Pop a value from the right (tail) of a list
    pub fn rpop(&self, key: &str) -> Option<Bytes> {
        let mut state = self.shared.lock().unwrap();

        state.entries.get_mut(key).and_then(|entry| {
            match &mut entry.value {
                Value::List(list) => list.pop_back(),
                _ => None,
            }
        })
    }

    /// Get a range of elements from a list
    pub fn lrange(&self, key: &str, start: isize, stop: isize) -> Option<Vec<Bytes>> {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::List(list) => {
                    let len = list.len() as isize;
                    
                    // Handle negative indices
                    let start = if start < 0 { (len + start).max(0) } else { start.min(len) } as usize;
                    let stop = if stop < 0 { (len + stop).max(-1) + 1 } else { (stop + 1).min(len) } as usize;

                    if start >= stop {
                        Some(Vec::new())
                    } else {
                        Some(list.iter().skip(start).take(stop - start).cloned().collect())
                    }
                }
                _ => None,
            }
        })
    }

    /// Get the length of a list
    pub fn llen(&self, key: &str) -> Option<usize> {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::List(list) => Some(list.len()),
                _ => None,
            }
        })
    }

    // ===== Set Operations =====

    /// Add members to a set
    pub fn sadd(&self, key: String, members: Vec<String>) -> usize {
        let mut state = self.shared.lock().unwrap();

        let entry = state.entries.entry(key).or_insert_with(|| Entry {
            value: Value::Set(HashSet::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::Set(set) => {
                let mut added = 0;
                for member in members {
                    if set.insert(member) {
                        added += 1;
                    }
                }
                added
            }
            _ => 0,
        }
    }

    /// Remove members from a set
    pub fn srem(&self, key: &str, members: Vec<String>) -> usize {
        let mut state = self.shared.lock().unwrap();

        state.entries.get_mut(key).map(|entry| {
            match &mut entry.value {
                Value::Set(set) => {
                    let mut removed = 0;
                    for member in members {
                        if set.remove(&member) {
                            removed += 1;
                        }
                    }
                    removed
                }
                _ => 0,
            }
        }).unwrap_or(0)
    }

    /// Get all members of a set
    pub fn smembers(&self, key: &str) -> Option<Vec<String>> {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::Set(set) => Some(set.iter().cloned().collect()),
                _ => None,
            }
        })
    }

    /// Check if a member exists in a set
    pub fn sismember(&self, key: &str, member: &str) -> bool {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).map(|entry| {
            match &entry.value {
                Value::Set(set) => set.contains(member),
                _ => false,
            }
        }).unwrap_or(false)
    }

    /// Get the cardinality (size) of a set
    pub fn scard(&self, key: &str) -> usize {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).map(|entry| {
            match &entry.value {
                Value::Set(set) => set.len(),
                _ => 0,
            }
        }).unwrap_or(0)
    }

    // ===== Hash Operations =====

    /// Set a field in a hash
    pub fn hset(&self, key: String, field: String, value: Bytes) -> bool {
        let mut state = self.shared.lock().unwrap();

        let entry = state.entries.entry(key).or_insert_with(|| Entry {
            value: Value::Hash(HashMap::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::Hash(hash) => {
                hash.insert(field, value).is_none()
            }
            _ => false,
        }
    }

    /// Get a field from a hash
    pub fn hget(&self, key: &str, field: &str) -> Option<Bytes> {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::Hash(hash) => hash.get(field).cloned(),
                _ => None,
            }
        })
    }

    /// Get all fields and values from a hash
    pub fn hgetall(&self, key: &str) -> Option<Vec<(String, Bytes)>> {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::Hash(hash) => {
                    Some(hash.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                }
                _ => None,
            }
        })
    }

    /// Delete a field from a hash
    pub fn hdel(&self, key: &str, fields: Vec<String>) -> usize {
        let mut state = self.shared.lock().unwrap();

        state.entries.get_mut(key).map(|entry| {
            match &mut entry.value {
                Value::Hash(hash) => {
                    let mut deleted = 0;
                    for field in fields {
                        if hash.remove(&field).is_some() {
                            deleted += 1;
                        }
                    }
                    deleted
                }
                _ => 0,
            }
        }).unwrap_or(0)
    }

    /// Check if a field exists in a hash
    pub fn hexists(&self, key: &str, field: &str) -> bool {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).map(|entry| {
            match &entry.value {
                Value::Hash(hash) => hash.contains_key(field),
                _ => false,
            }
        }).unwrap_or(false)
    }

    /// Get the number of fields in a hash
    pub fn hlen(&self, key: &str) -> usize {
        let state = self.shared.lock().unwrap();

        state.entries.get(key).map(|entry| {
            match &entry.value {
                Value::Hash(hash) => hash.len(),
                _ => 0,
            }
        }).unwrap_or(0)
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
