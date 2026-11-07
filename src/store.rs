//! Storage abstraction shared by the single-mutex [`Db`] and the sharded
//! [`DbDashMap`], so the server and command layer can run on either one.

use crate::db::Db;
use crate::db_dashmap::DbDashMap;
use bytes::Bytes;
use std::time::Instant;

/// The keyspace operations the command layer needs.
pub trait Store: Clone + Send + Sync + 'static {
    fn new() -> Self;
    fn read_string(&self, key: &[u8]) -> Option<Bytes>;
    fn write_string(&self, key: Bytes, value: Bytes, expires_at: Option<Instant>);
    fn get_type(&self, key: &[u8]) -> Option<&'static str>;
    fn exists(&self, key: &[u8]) -> bool;
    fn delete(&self, key: &[u8]) -> bool;
    fn lpush(&self, key: Bytes, values: Vec<Bytes>) -> usize;
    fn rpush(&self, key: Bytes, values: Vec<Bytes>) -> usize;
    fn lpop(&self, key: &[u8]) -> Option<Bytes>;
    fn rpop(&self, key: &[u8]) -> Option<Bytes>;
    fn lrange(&self, key: &[u8], start: isize, stop: isize) -> Option<Vec<Bytes>>;
    fn llen(&self, key: &[u8]) -> Option<usize>;
    fn sadd(&self, key: Bytes, members: Vec<Bytes>) -> usize;
    fn srem(&self, key: &[u8], members: Vec<Bytes>) -> usize;
    fn smembers(&self, key: &[u8]) -> Option<Vec<Bytes>>;
    fn sismember(&self, key: &[u8], member: &[u8]) -> bool;
    fn scard(&self, key: &[u8]) -> usize;
    fn hset(&self, key: Bytes, field: Bytes, value: Bytes) -> bool;
    fn hget(&self, key: &[u8], field: &[u8]) -> Option<Bytes>;
    fn hgetall(&self, key: &[u8]) -> Option<Vec<(Bytes, Bytes)>>;
    fn hdel(&self, key: &[u8], fields: Vec<Bytes>) -> usize;
    fn hexists(&self, key: &[u8], field: &[u8]) -> bool;
    fn hlen(&self, key: &[u8]) -> usize;
    fn dbsize(&self) -> usize;
    fn flushdb(&self);
    fn keys(&self, pattern: &str) -> Vec<Bytes>;
}

macro_rules! impl_store {
    ($ty:ty) => {
        impl Store for $ty {
            fn new() -> Self {
                <$ty>::new()
            }
            fn read_string(&self, key: &[u8]) -> Option<Bytes> {
                <$ty>::read_string(self, key)
            }
            fn write_string(&self, key: Bytes, value: Bytes, expires_at: Option<Instant>) {
                <$ty>::write_string(self, key, value, expires_at)
            }
            fn get_type(&self, key: &[u8]) -> Option<&'static str> {
                <$ty>::get_type(self, key)
            }
            fn exists(&self, key: &[u8]) -> bool {
                <$ty>::exists(self, key)
            }
            fn delete(&self, key: &[u8]) -> bool {
                <$ty>::delete(self, key)
            }
            fn lpush(&self, key: Bytes, values: Vec<Bytes>) -> usize {
                <$ty>::lpush(self, key, values)
            }
            fn rpush(&self, key: Bytes, values: Vec<Bytes>) -> usize {
                <$ty>::rpush(self, key, values)
            }
            fn lpop(&self, key: &[u8]) -> Option<Bytes> {
                <$ty>::lpop(self, key)
            }
            fn rpop(&self, key: &[u8]) -> Option<Bytes> {
                <$ty>::rpop(self, key)
            }
            fn lrange(&self, key: &[u8], start: isize, stop: isize) -> Option<Vec<Bytes>> {
                <$ty>::lrange(self, key, start, stop)
            }
            fn llen(&self, key: &[u8]) -> Option<usize> {
                <$ty>::llen(self, key)
            }
            fn sadd(&self, key: Bytes, members: Vec<Bytes>) -> usize {
                <$ty>::sadd(self, key, members)
            }
            fn srem(&self, key: &[u8], members: Vec<Bytes>) -> usize {
                <$ty>::srem(self, key, members)
            }
            fn smembers(&self, key: &[u8]) -> Option<Vec<Bytes>> {
                <$ty>::smembers(self, key)
            }
            fn sismember(&self, key: &[u8], member: &[u8]) -> bool {
                <$ty>::sismember(self, key, member)
            }
            fn scard(&self, key: &[u8]) -> usize {
                <$ty>::scard(self, key)
            }
            fn hset(&self, key: Bytes, field: Bytes, value: Bytes) -> bool {
                <$ty>::hset(self, key, field, value)
            }
            fn hget(&self, key: &[u8], field: &[u8]) -> Option<Bytes> {
                <$ty>::hget(self, key, field)
            }
            fn hgetall(&self, key: &[u8]) -> Option<Vec<(Bytes, Bytes)>> {
                <$ty>::hgetall(self, key)
            }
            fn hdel(&self, key: &[u8], fields: Vec<Bytes>) -> usize {
                <$ty>::hdel(self, key, fields)
            }
            fn hexists(&self, key: &[u8], field: &[u8]) -> bool {
                <$ty>::hexists(self, key, field)
            }
            fn hlen(&self, key: &[u8]) -> usize {
                <$ty>::hlen(self, key)
            }
            fn dbsize(&self) -> usize {
                <$ty>::dbsize(self)
            }
            fn flushdb(&self) {
                <$ty>::flushdb(self)
            }
            fn keys(&self, pattern: &str) -> Vec<Bytes> {
                <$ty>::keys(self, pattern)
            }
        }
    };
}

impl_store!(Db);
impl_store!(DbDashMap);
