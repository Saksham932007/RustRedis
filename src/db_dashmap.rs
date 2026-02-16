use bytes::Bytes;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

/// Value types supported by the database (same as db.rs)
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

/// Database entry with optional expiration
struct Entry {
    value: Value,
    expires_at: Option<Instant>,
}

impl Entry {
    /// Check if this entry has expired
    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|t| Instant::now() >= t)
            .unwrap_or(false)
    }
}

/// Lock-free database using DashMap.
///
/// DashMap uses sharded locking internally — it partitions the hash table
/// into N shards (typically = number of CPU cores) and locks only the
/// relevant shard during writes. This provides:
///
/// - **Concurrent reads**: Multiple readers on different shards proceed without contention
/// - **Reduced write contention**: Writers only block writers to the same shard
/// - **Better scaling**: Near-linear throughput scaling with core count
///
/// Compared to `Arc<Mutex<HashMap>>`, this eliminates the global lock bottleneck
/// where ANY read or write blocks ALL other operations.
#[derive(Clone)]
pub struct DbDashMap {
    entries: Arc<DashMap<String, Entry>>,
}

impl DbDashMap {
    /// Create a new DashMap-backed database instance
    pub fn new() -> DbDashMap {
        DbDashMap {
            entries: Arc::new(DashMap::new()),
        }
    }

    /// Helper: remove entry if expired, returning true if it was removed
    fn remove_if_expired(&self, key: &str) -> bool {
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                drop(entry); // Release read reference before removing
                self.entries.remove(key);
                return true;
            }
        }
        false
    }

    /// Read a String value from the database
    pub fn read_string(&self, key: &str) -> Option<Bytes> {
        // Check expiration first
        if self.remove_if_expired(key) {
            return None;
        }

        self.entries.get(key).and_then(|entry| match &entry.value {
            Value::String(bytes) => Some(bytes.clone()),
            _ => None,
        })
    }

    /// Write a String value to the database with optional expiration
    pub fn write_string(&self, key: String, value: Bytes, expires_at: Option<Instant>) {
        self.entries.insert(
            key,
            Entry {
                value: Value::String(value),
                expires_at,
            },
        );
    }

    /// Get the type of a value
    pub fn get_type(&self, key: &str) -> Option<&'static str> {
        self.entries.get(key).map(|entry| entry.value.type_name())
    }

    /// Check if a key exists (and hasn't expired)
    pub fn exists(&self, key: &str) -> bool {
        if self.remove_if_expired(key) {
            return false;
        }
        self.entries.contains_key(key)
    }

    /// Delete a key from the database
    pub fn delete(&self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    // ===== List Operations =====

    pub fn lpush(&self, key: String, values: Vec<Bytes>) -> usize {
        let mut entry = self.entries.entry(key).or_insert_with(|| Entry {
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
            _ => 0,
        }
    }

    pub fn rpush(&self, key: String, values: Vec<Bytes>) -> usize {
        let mut entry = self.entries.entry(key).or_insert_with(|| Entry {
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

    pub fn lpop(&self, key: &str) -> Option<Bytes> {
        self.entries
            .get_mut(key)?
            .value_mut()
            .as_list_mut()
            .and_then(|list| list.pop_front())
    }

    pub fn rpop(&self, key: &str) -> Option<Bytes> {
        self.entries
            .get_mut(key)?
            .value_mut()
            .as_list_mut()
            .and_then(|list| list.pop_back())
    }

    pub fn lrange(&self, key: &str, start: isize, stop: isize) -> Option<Vec<Bytes>> {
        self.entries.get(key).and_then(|entry| {
            match &entry.value {
                Value::List(list) => {
                    let len = list.len() as isize;
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

    pub fn llen(&self, key: &str) -> Option<usize> {
        self.entries.get(key).and_then(|entry| match &entry.value {
            Value::List(list) => Some(list.len()),
            _ => None,
        })
    }

    // ===== Set Operations =====

    pub fn sadd(&self, key: String, members: Vec<String>) -> usize {
        let mut entry = self.entries.entry(key).or_insert_with(|| Entry {
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

    pub fn srem(&self, key: &str, members: Vec<String>) -> usize {
        self.entries
            .get_mut(key)
            .map(|mut entry| match &mut entry.value {
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

    pub fn smembers(&self, key: &str) -> Option<Vec<String>> {
        self.entries.get(key).and_then(|entry| match &entry.value {
            Value::Set(set) => Some(set.iter().cloned().collect()),
            _ => None,
        })
    }

    pub fn sismember(&self, key: &str, member: &str) -> bool {
        self.entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Set(set) => set.contains(member),
                _ => false,
            })
            .unwrap_or(false)
    }

    pub fn scard(&self, key: &str) -> usize {
        self.entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Set(set) => set.len(),
                _ => 0,
            })
            .unwrap_or(0)
    }

    // ===== Hash Operations =====

    pub fn hset(&self, key: String, field: String, value: Bytes) -> bool {
        let mut entry = self.entries.entry(key).or_insert_with(|| Entry {
            value: Value::Hash(HashMap::new()),
            expires_at: None,
        });

        match &mut entry.value {
            Value::Hash(hash) => hash.insert(field, value).is_none(),
            _ => false,
        }
    }

    pub fn hget(&self, key: &str, field: &str) -> Option<Bytes> {
        self.entries.get(key).and_then(|entry| match &entry.value {
            Value::Hash(hash) => hash.get(field).cloned(),
            _ => None,
        })
    }

    pub fn hgetall(&self, key: &str) -> Option<Vec<(String, Bytes)>> {
        self.entries.get(key).and_then(|entry| match &entry.value {
            Value::Hash(hash) => Some(hash.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            _ => None,
        })
    }

    pub fn hdel(&self, key: &str, fields: Vec<String>) -> usize {
        self.entries
            .get_mut(key)
            .map(|mut entry| match &mut entry.value {
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

    pub fn hexists(&self, key: &str, field: &str) -> bool {
        self.entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Hash(hash) => hash.contains_key(field),
                _ => false,
            })
            .unwrap_or(false)
    }

    pub fn hlen(&self, key: &str) -> usize {
        self.entries
            .get(key)
            .map(|entry| match &entry.value {
                Value::Hash(hash) => hash.len(),
                _ => 0,
            })
            .unwrap_or(0)
    }

    // ===== Database Utility Operations =====

    pub fn dbsize(&self) -> usize {
        self.entries.len()
    }

    pub fn flushdb(&self) {
        self.entries.clear();
    }

    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let regex_pattern = Self::glob_to_regex(pattern);
        let re = match regex::Regex::new(&regex_pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        self.entries
            .iter()
            .filter(|entry| re.is_match(entry.key()))
            .map(|entry| entry.key().clone())
            .collect()
    }

    fn glob_to_regex(pattern: &str) -> String {
        let mut regex = String::from("^");
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

impl Default for DbDashMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for Value mutations through DashMap refs
trait ValueMut {
    fn as_list_mut(&mut self) -> Option<&mut VecDeque<Bytes>>;
}

impl ValueMut for Value {
    fn as_list_mut(&mut self) -> Option<&mut VecDeque<Bytes>> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }
}

// We need ValueMut on Entry too for DashMap::get_mut()
impl ValueMut for Entry {
    fn as_list_mut(&mut self) -> Option<&mut VecDeque<Bytes>> {
        self.value.as_list_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashmap_string_operations() {
        let db = DbDashMap::new();

        db.write_string("key1".to_string(), Bytes::from("value1"), None);
        assert_eq!(db.read_string("key1").unwrap(), Bytes::from("value1"));
        assert!(db.read_string("nonexistent").is_none());
    }

    #[test]
    fn test_dashmap_list_operations() {
        let db = DbDashMap::new();

        let len = db.lpush(
            "mylist".to_string(),
            vec![Bytes::from("a"), Bytes::from("b")],
        );
        assert_eq!(len, 2);

        let len = db.rpush("mylist".to_string(), vec![Bytes::from("c")]);
        assert_eq!(len, 3);

        let range = db.lrange("mylist", 0, -1).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0], Bytes::from("a"));
        assert_eq!(range[1], Bytes::from("b"));
        assert_eq!(range[2], Bytes::from("c"));

        let value = db.lpop("mylist").unwrap();
        assert_eq!(value, Bytes::from("a"));

        assert_eq!(db.llen("mylist").unwrap(), 2);
    }

    #[test]
    fn test_dashmap_set_operations() {
        let db = DbDashMap::new();

        let added = db.sadd(
            "myset".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(added, 3);

        assert!(db.sismember("myset", "a"));
        assert!(!db.sismember("myset", "d"));
        assert_eq!(db.scard("myset"), 3);

        let removed = db.srem("myset", vec!["b".to_string()]);
        assert_eq!(removed, 1);
        assert_eq!(db.scard("myset"), 2);
    }

    #[test]
    fn test_dashmap_hash_operations() {
        let db = DbDashMap::new();

        let is_new = db.hset(
            "user:1".to_string(),
            "name".to_string(),
            Bytes::from("Alice"),
        );
        assert!(is_new);

        let value = db.hget("user:1", "name").unwrap();
        assert_eq!(value, Bytes::from("Alice"));

        assert!(db.hexists("user:1", "name"));
        assert!(!db.hexists("user:1", "age"));

        db.hset("user:1".to_string(), "age".to_string(), Bytes::from("30"));
        assert_eq!(db.hlen("user:1"), 2);

        let deleted = db.hdel("user:1", vec!["age".to_string()]);
        assert_eq!(deleted, 1);
        assert_eq!(db.hlen("user:1"), 1);
    }

    #[test]
    fn test_dashmap_utility_operations() {
        let db = DbDashMap::new();

        db.write_string("key1".to_string(), Bytes::from("val1"), None);
        db.write_string("key2".to_string(), Bytes::from("val2"), None);
        db.lpush("list1".to_string(), vec![Bytes::from("item")]);

        assert_eq!(db.dbsize(), 3);
        assert!(db.exists("key1"));
        assert!(!db.exists("nonexistent"));

        assert_eq!(db.get_type("key1"), Some("string"));
        assert_eq!(db.get_type("list1"), Some("list"));

        assert!(db.delete("key1"));
        assert_eq!(db.dbsize(), 2);

        db.flushdb();
        assert_eq!(db.dbsize(), 0);
    }

    #[test]
    fn test_dashmap_expiration() {
        use std::time::Duration;

        let db = DbDashMap::new();

        let expires_at = Instant::now() + Duration::from_millis(100);
        db.write_string("temp".to_string(), Bytes::from("value"), Some(expires_at));

        assert!(db.read_string("temp").is_some());

        std::thread::sleep(Duration::from_millis(150));

        assert!(db.read_string("temp").is_none());
    }

    #[test]
    fn test_dashmap_concurrent_writes() {
        use std::sync::Arc;
        use std::thread;

        let db = Arc::new(DbDashMap::new());
        let mut handles = vec![];

        // Spawn 10 threads, each writing 100 keys
        for t in 0..10 {
            let db = Arc::clone(&db);
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    db.write_string(
                        format!("key:{}:{}", t, i),
                        Bytes::from(format!("value:{}:{}", t, i)),
                        None,
                    );
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(db.dbsize(), 1000);
    }
}
