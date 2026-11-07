use crate::sync::LockExt;
use bytes::Bytes;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Value types supported by the database
#[derive(Clone, Debug)]
pub enum Value {
    String(Bytes),
    List(VecDeque<Bytes>),
    Set(HashSet<Bytes>),
    Hash(HashMap<Bytes, Bytes>),
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
    entries: HashMap<Bytes, Entry>,
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
    pub fn read_string(&self, key: &[u8]) -> Option<Bytes> {
        let mut state = self.shared.lock_recover();

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
    pub fn write_string(&self, key: Bytes, value: Bytes, expires_at: Option<Instant>) {
        let mut state = self.shared.lock_recover();

        let entry = Entry {
            value: Value::String(value),
            expires_at,
        };

        state.entries.insert(key, entry);
    }

    /// Get the type of a value
    pub fn get_type(&self, key: &[u8]) -> Option<&'static str> {
        let state = self.shared.lock_recover();
        state.entries.get(key).map(|entry| entry.value.type_name())
    }

    /// Check if a key exists (and hasn't expired)
    pub fn exists(&self, key: &[u8]) -> bool {
        let mut state = self.shared.lock_recover();

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
    pub fn delete(&self, key: &[u8]) -> bool {
        let mut state = self.shared.lock_recover();
        state.entries.remove(key).is_some()
    }

    // ===== List Operations =====

    /// Push values to the left (head) of a list
    pub fn lpush(&self, key: Bytes, values: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock_recover();

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
    pub fn rpush(&self, key: Bytes, values: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock_recover();

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
    pub fn lpop(&self, key: &[u8]) -> Option<Bytes> {
        let mut state = self.shared.lock_recover();

        state
            .entries
            .get_mut(key)
            .and_then(|entry| match &mut entry.value {
                Value::List(list) => list.pop_front(),
                _ => None,
            })
    }

    /// Pop a value from the right (tail) of a list
    pub fn rpop(&self, key: &[u8]) -> Option<Bytes> {
        let mut state = self.shared.lock_recover();

        state
            .entries
            .get_mut(key)
            .and_then(|entry| match &mut entry.value {
                Value::List(list) => list.pop_back(),
                _ => None,
            })
    }

    /// Get a range of elements from a list
    pub fn lrange(&self, key: &[u8], start: isize, stop: isize) -> Option<Vec<Bytes>> {
        let state = self.shared.lock_recover();

        state.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::List(list) => {
                    let len = list.len() as isize;

                    // Handle negative indices
                    let start = if start < 0 {
                        (len + start).max(0)
                    } else {
                        start.min(len)
                    } as usize;
                    let stop = if stop < 0 {
                        (len + stop).max(-1) + 1
                    } else {
                        (stop + 1).min(len)
                    } as usize;

                    if start >= stop {
                        Some(Vec::new())
                    } else {
                        Some(
                            list.iter()
                                .skip(start)
                                .take(stop - start)
                                .cloned()
                                .collect(),
                        )
                    }
                }
                _ => None,
            }
        })
    }

    /// Get the length of a list
    pub fn llen(&self, key: &[u8]) -> Option<usize> {
        let state = self.shared.lock_recover();

        state.entries.get(key).and_then(|entry| match &entry.value {
            Value::List(list) => Some(list.len()),
            _ => None,
        })
    }

    // ===== Set Operations =====

    /// Add members to a set
    pub fn sadd(&self, key: Bytes, members: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock_recover();

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
    pub fn srem(&self, key: &[u8], members: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock_recover();

        state
            .entries
            .get_mut(key)
            .map(|entry| match &mut entry.value {
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
            })
            .unwrap_or(0)
    }

    /// Get all members of a set
    pub fn smembers(&self, key: &[u8]) -> Option<Vec<Bytes>> {
        let state = self.shared.lock_recover();

        state.entries.get(key).and_then(|entry| match &entry.value {
            Value::Set(set) => Some(set.iter().cloned().collect()),
            _ => None,
        })
    }

    /// Check if a member exists in a set
    pub fn sismember(&self, key: &[u8], member: &[u8]) -> bool {
        let state = self.shared.lock_recover();

        state
            .entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Set(set) => set.contains(member),
                _ => false,
            })
            .unwrap_or(false)
    }

    /// Get the cardinality (size) of a set
    pub fn scard(&self, key: &[u8]) -> usize {
        let state = self.shared.lock_recover();

        state
            .entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Set(set) => set.len(),
                _ => 0,
            })
            .unwrap_or(0)
    }

    // ===== Hash Operations =====

    /// Set a field in a hash
    pub fn hset(&self, key: Bytes, field: Bytes, value: Bytes) -> bool {
        let mut state = self.shared.lock_recover();

        let entry = state.entries.entry(key).or_insert_with(|| Entry {
            value: Value::Hash(HashMap::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::Hash(hash) => hash.insert(field, value).is_none(),
            _ => false,
        }
    }

    /// Get a field from a hash
    pub fn hget(&self, key: &[u8], field: &[u8]) -> Option<Bytes> {
        let state = self.shared.lock_recover();

        state.entries.get(key).and_then(|entry| match &entry.value {
            Value::Hash(hash) => hash.get(field).cloned(),
            _ => None,
        })
    }

    /// Get all fields and values from a hash
    pub fn hgetall(&self, key: &[u8]) -> Option<Vec<(Bytes, Bytes)>> {
        let state = self.shared.lock_recover();

        state.entries.get(key).and_then(|entry| match &entry.value {
            Value::Hash(hash) => Some(hash.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            _ => None,
        })
    }

    /// Delete a field from a hash
    pub fn hdel(&self, key: &[u8], fields: Vec<Bytes>) -> usize {
        let mut state = self.shared.lock_recover();

        state
            .entries
            .get_mut(key)
            .map(|entry| match &mut entry.value {
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
            })
            .unwrap_or(0)
    }

    /// Check if a field exists in a hash
    pub fn hexists(&self, key: &[u8], field: &[u8]) -> bool {
        let state = self.shared.lock_recover();

        state
            .entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Hash(hash) => hash.contains_key(field),
                _ => false,
            })
            .unwrap_or(false)
    }

    /// Get the number of fields in a hash
    pub fn hlen(&self, key: &[u8]) -> usize {
        let state = self.shared.lock_recover();

        state
            .entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Hash(hash) => hash.len(),
                _ => 0,
            })
            .unwrap_or(0)
    }

    // ===== Database Utility Operations =====

    /// Get the total number of keys in the database
    pub fn dbsize(&self) -> usize {
        let state = self.shared.lock_recover();
        state.entries.len()
    }

    /// Clear all keys from the database
    pub fn flushdb(&self) {
        let mut state = self.shared.lock_recover();
        state.entries.clear();
    }

    /// Get all keys matching a pattern
    ///
    /// Supports simple glob-style patterns:
    /// - h?llo matches hello, hallo, hxllo
    /// - h*llo matches hllo, heeeello
    /// - h[ae]llo matches hello and hallo
    pub fn keys(&self, pattern: &str) -> Vec<Bytes> {
        let state = self.shared.lock_recover();

        // Convert glob pattern to regex
        let regex_pattern = Self::glob_to_regex(pattern);
        let re = match regex::bytes::Regex::new(&regex_pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        state
            .entries
            .keys()
            .filter(|key| re.is_match(key))
            .cloned()
            .collect()
    }

    /// Convert a glob pattern to a regex pattern
    fn glob_to_regex(pattern: &str) -> String {
        let mut regex = String::from("(?s-u)^");
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '*' => regex.push_str(".*"),
                '?' => regex.push('.'),
                '[' => {
                    regex.push('[');
                    while let Some(&next_c) = chars.peek() {
                        chars.next();
                        regex.push(next_c);
                        if next_c == ']' {
                            break;
                        }
                    }
                }
                '.' | '+' | '^' | '$' | '(' | ')' | '{' | '}' | '|' | '\\' => {
                    regex.push('\\');
                    regex.push(c);
                }
                _ => regex.push(c),
            }
        }

        regex.push('$');
        regex
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
