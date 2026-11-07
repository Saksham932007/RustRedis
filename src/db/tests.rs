    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_string_operations() {
        let db = Db::new();

        // Test write and read
        db.write_string(Bytes::from("key1"), Bytes::from("value1"), None);
        assert_eq!(
            db.read_string(b"key1").unwrap(),
            Bytes::from("value1")
        );

        // Test non-existent key
        assert!(db.read_string(b"nonexistent").is_none());
    }

    #[test]
    fn test_list_operations() {
        let db = Db::new();

        // Test LPUSH
        // Values are reversed, so [a, b] becomes [b, a]
        // Then b is pushed to front, then a is pushed to front
        // Result: [a, b] (a at head)
        let len = db.lpush(
            Bytes::from("mylist"),
            vec![Bytes::from("a"), Bytes::from("b")],
        );
        assert_eq!(len, 2);

        // Test RPUSH - adds to tail
        let len = db.rpush(Bytes::from("mylist"), vec![Bytes::from("c")]);
        assert_eq!(len, 3);

        // Test LRANGE - list is now [a, b, c]
        let range = db.lrange(b"mylist", 0, -1).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0], Bytes::from("a"));
        assert_eq!(range[1], Bytes::from("b"));
        assert_eq!(range[2], Bytes::from("c"));

        // Test LPOP - removes from head (a)
        let value = db.lpop(b"mylist").unwrap();
        assert_eq!(value, Bytes::from("a"));

        // Test LLEN - should have 2 items left
        assert_eq!(db.llen(b"mylist").unwrap(), 2);
    }

    #[test]
    fn test_set_operations() {
        let db = Db::new();

        // Test SADD
        let added = db.sadd(
            Bytes::from("myset"),
            vec![Bytes::from("a"), Bytes::from("b"), Bytes::from("c")],
        );
        assert_eq!(added, 3);

        // Test SISMEMBER
        assert!(db.sismember(b"myset", b"a"));
        assert!(!db.sismember(b"myset", b"d"));

        // Test SCARD
        assert_eq!(db.scard(b"myset"), 3);

        // Test SREM
        let removed = db.srem(b"myset", vec![Bytes::from("b")]);
        assert_eq!(removed, 1);
        assert_eq!(db.scard(b"myset"), 2);
    }

    #[test]
    fn test_hash_operations() {
        let db = Db::new();

        // Test HSET
        let is_new = db.hset(
            Bytes::from("user:1"),
            Bytes::from("name"),
            Bytes::from("Alice"),
        );
        assert!(is_new);

        // Test HGET
        let value = db.hget(b"user:1", b"name").unwrap();
        assert_eq!(value, Bytes::from("Alice"));

        // Test HEXISTS
        assert!(db.hexists(b"user:1", b"name"));
        assert!(!db.hexists(b"user:1", b"age"));

        // Test HLEN
        db.hset(Bytes::from("user:1"), Bytes::from("age"), Bytes::from("30"));
        assert_eq!(db.hlen(b"user:1"), 2);

        // Test HDEL
        let deleted = db.hdel(b"user:1", vec![Bytes::from("age")]);
        assert_eq!(deleted, 1);
        assert_eq!(db.hlen(b"user:1"), 1);
    }

    #[test]
    fn test_utility_operations() {
        let db = Db::new();

        // Add some keys
        db.write_string(Bytes::from("key1"), Bytes::from("val1"), None);
        db.write_string(Bytes::from("key2"), Bytes::from("val2"), None);
        db.lpush(Bytes::from("list1"), vec![Bytes::from("item")]);

        // Test DBSIZE
        assert_eq!(db.dbsize(), 3);

        // Test EXISTS
        assert!(db.exists(b"key1"));
        assert!(!db.exists(b"nonexistent"));

        // Test TYPE
        assert_eq!(db.get_type(b"key1"), Some("string"));
        assert_eq!(db.get_type(b"list1"), Some("list"));
        assert_eq!(db.get_type(b"nonexistent"), None);

        // Test DEL
        assert!(db.delete(b"key1"));
        assert!(!db.delete(b"nonexistent"));
        assert_eq!(db.dbsize(), 2);

        // Test FLUSHDB
        db.flushdb();
        assert_eq!(db.dbsize(), 0);
    }

    #[test]
    fn test_keys_pattern_matching() {
        let db = Db::new();

        // Add various keys
        db.write_string(Bytes::from("user:1"), Bytes::from("a"), None);
        db.write_string(Bytes::from("user:2"), Bytes::from("b"), None);
        db.write_string(Bytes::from("session:1"), Bytes::from("c"), None);
        db.write_string(Bytes::from("data"), Bytes::from("d"), None);

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
        db.write_string(Bytes::from("temp"), Bytes::from("value"), Some(expires_at));

        // Should exist immediately
        assert!(db.read_string(b"temp").is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Should be expired and return None
        assert!(db.read_string(b"temp").is_none());
    }

    #[test]
    fn test_type_safety() {
        let db = Db::new();

        // Create a list
        db.lpush(Bytes::from("mylist"), vec![Bytes::from("item")]);

        // Try to read as string - should return None
        assert!(db.read_string(b"mylist").is_none());

        // Type should be "list"
        assert_eq!(db.get_type(b"mylist"), Some("list"));
    }

    #[test]
    fn test_db_survives_a_panic_while_the_lock_is_held() {
        let db = Db::new();
        db.write_string(Bytes::from("key"), Bytes::from("value"), None);

        let poisoner = db.clone();
        let result = std::thread::spawn(move || {
            let _guard = poisoner.shared.lock().unwrap();
            panic!("simulated bug while holding the keyspace lock");
        })
        .join();
        assert!(result.is_err());
        assert!(db.shared.is_poisoned());

        // The database keeps serving requests instead of panicking forever
        assert_eq!(db.read_string(b"key").unwrap(), Bytes::from("value"));
        db.write_string(Bytes::from("other"), Bytes::from("x"), None);
        assert_eq!(db.read_string(b"other").unwrap(), Bytes::from("x"));
    }

    #[test]
    fn test_binary_keys_members_and_fields() {
        let db = Db::new();
        let key: &[u8] = b"\xff\xfe\x00key";

        db.write_string(Bytes::copy_from_slice(key), Bytes::from("v"), None);
        assert_eq!(db.read_string(key).unwrap(), Bytes::from("v"));
        assert!(db.exists(key));

        db.sadd(Bytes::from("set"), vec![Bytes::copy_from_slice(b"\x80\x81")]);
        assert!(db.sismember(b"set", b"\x80\x81"));

        db.hset(
            Bytes::from("hash"),
            Bytes::copy_from_slice(b"\xc3\x28"),
            Bytes::from("x"),
        );
        assert_eq!(db.hget(b"hash", b"\xc3\x28").unwrap(), Bytes::from("x"));

        assert!(db.keys("*").contains(&Bytes::copy_from_slice(key)));
        assert!(db.delete(key));
        assert!(!db.exists(key));
    }
