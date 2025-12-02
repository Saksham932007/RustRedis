#[cfg(test)]
mod tests {
    use super::super::*;
    use bytes::Bytes;

    #[test]
    fn test_string_operations() {
        let db = Db::new();

        // Test write and read
        db.write_string("key1".to_string(), Bytes::from("value1"), None);
        assert_eq!(
            db.read_string("key1").unwrap(),
            Bytes::from("value1")
        );

        // Test non-existent key
        assert!(db.read_string("nonexistent").is_none());
    }

    #[test]
    fn test_list_operations() {
        let db = Db::new();

        // Test LPUSH
        // Values are reversed, so [a, b] becomes [b, a]
        // Then b is pushed to front, then a is pushed to front
        // Result: [a, b] (a at head)
        let len = db.lpush(
            "mylist".to_string(),
            vec![Bytes::from("a"), Bytes::from("b")],
        );
        assert_eq!(len, 2);

        // Test RPUSH - adds to tail
        let len = db.rpush("mylist".to_string(), vec![Bytes::from("c")]);
        assert_eq!(len, 3);

        // Test LRANGE - list is now [a, b, c]
        let range = db.lrange("mylist", 0, -1).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0], Bytes::from("a"));
        assert_eq!(range[1], Bytes::from("b"));
        assert_eq!(range[2], Bytes::from("c"));

        // Test LPOP - removes from head (a)
        let value = db.lpop("mylist").unwrap();
        assert_eq!(value, Bytes::from("a"));

        // Test LLEN - should have 2 items left
        assert_eq!(db.llen("mylist").unwrap(), 2);
    }

    #[test]
    fn test_set_operations() {
        let db = Db::new();

        // Test SADD
        let added = db.sadd(
            "myset".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(added, 3);

        // Test SISMEMBER
        assert!(db.sismember("myset", "a"));
        assert!(!db.sismember("myset", "d"));

        // Test SCARD
        assert_eq!(db.scard("myset"), 3);

        // Test SREM
        let removed = db.srem("myset", vec!["b".to_string()]);
        assert_eq!(removed, 1);
        assert_eq!(db.scard("myset"), 2);
    }

    #[test]
    fn test_hash_operations() {
        let db = Db::new();

        // Test HSET
        let is_new = db.hset(
            "user:1".to_string(),
            "name".to_string(),
            Bytes::from("Alice"),
        );
        assert!(is_new);

        // Test HGET
        let value = db.hget("user:1", "name").unwrap();
        assert_eq!(value, Bytes::from("Alice"));

        // Test HEXISTS
        assert!(db.hexists("user:1", "name"));
        assert!(!db.hexists("user:1", "age"));

        // Test HLEN
        db.hset("user:1".to_string(), "age".to_string(), Bytes::from("30"));
        assert_eq!(db.hlen("user:1"), 2);

        // Test HDEL
        let deleted = db.hdel("user:1", vec!["age".to_string()]);
        assert_eq!(deleted, 1);
        assert_eq!(db.hlen("user:1"), 1);
    }

    #[test]
    fn test_utility_operations() {
        let db = Db::new();

        // Add some keys
        db.write_string("key1".to_string(), Bytes::from("val1"), None);
        db.write_string("key2".to_string(), Bytes::from("val2"), None);
        db.lpush("list1".to_string(), vec![Bytes::from("item")]);

        // Test DBSIZE
        assert_eq!(db.dbsize(), 3);

        // Test EXISTS
        assert!(db.exists("key1"));
        assert!(!db.exists("nonexistent"));

        // Test TYPE
        assert_eq!(db.get_type("key1"), Some("string"));
        assert_eq!(db.get_type("list1"), Some("list"));
        assert_eq!(db.get_type("nonexistent"), None);

        // Test DEL
        assert!(db.delete("key1"));
        assert!(!db.delete("nonexistent"));
        assert_eq!(db.dbsize(), 2);

        // Test FLUSHDB
        db.flushdb();
        assert_eq!(db.dbsize(), 0);
    }

    #[test]
    fn test_keys_pattern_matching() {
        let db = Db::new();

        // Add various keys
        db.write_string("user:1".to_string(), Bytes::from("a"), None);
        db.write_string("user:2".to_string(), Bytes::from("b"), None);
        db.write_string("session:1".to_string(), Bytes::from("c"), None);
        db.write_string("data".to_string(), Bytes::from("d"), None);

        // Test wildcard pattern
        let keys = db.keys("user:*");
        assert_eq!(keys.len(), 2);

        // Test all keys
        let all_keys = db.keys("*");
        assert_eq!(all_keys.len(), 4);

        // Test single char wildcard
        let keys = db.keys("user:?");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_expiration() {
        let db = Db::new();
        use std::time::{Duration, Instant};

        // Set a key with 1 second expiration
        let expires_at = Instant::now() + Duration::from_millis(100);
        db.write_string("temp".to_string(), Bytes::from("value"), Some(expires_at));

        // Should exist immediately
        assert!(db.read_string("temp").is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Should be expired and return None
        assert!(db.read_string("temp").is_none());
    }

    #[test]
    fn test_type_safety() {
        let db = Db::new();

        // Create a list
        db.lpush("mylist".to_string(), vec![Bytes::from("item")]);

        // Try to read as string - should return None
        assert!(db.read_string("mylist").is_none());

        // Type should be "list"
        assert_eq!(db.get_type("mylist"), Some("list"));
    }
}
