# RustRedis

A high-performance Redis clone implemented in Rust, featuring full RESP (Redis Serialization Protocol) support, multiple data structures, persistence, and Pub/Sub capabilities.

![Rust](https://img.shields.io/badge/rust-2021-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)

## ✨ Features

### Core Features
- ✅ **Full RESP Protocol Support** - All 6 RESP data types implemented
- ✅ **Async I/O** - Built on Tokio for high-performance concurrent connections
- ✅ **Multiple Data Structures** - Strings, Lists, Sets, and Hashes
- ✅ **TTL/Expiration** - Keys can expire automatically with lazy cleanup
- ✅ **Thread-Safe** - Shared state using `Arc<Mutex<HashMap>>`
- ✅ **Zero-Copy** - Efficient byte handling with the `bytes` crate

### Advanced Features
- ✅ **AOF Persistence** - Append-Only File for data durability
- ✅ **Pub/Sub Messaging** - Publish/Subscribe pattern support
- ✅ **Pattern Matching** - KEYS command with glob pattern support
- ✅ **Database Management** - DBSIZE, FLUSHDB commands
- ✅ **Structured Logging** - Production-ready observability with `tracing`
- ✅ **Idiomatic Rust** - Passes clippy, formatted with rustfmt

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/Saksham932007/RustRedis.git
cd RustRedis

# Build and run
cargo run --bin server

# In another terminal, connect with redis-cli
redis-cli -p 6379

# Try some commands!
127.0.0.1:6379> PING
PONG

# String operations
127.0.0.1:6379> SET mykey "Hello, RustRedis!"
OK
127.0.0.1:6379> GET mykey
"Hello, RustRedis!"

# List operations
127.0.0.1:6379> LPUSH mylist "world" "hello"
(integer) 2
127.0.0.1:6379> LRANGE mylist 0 -1
1) "hello"
2) "world"

# Set operations
127.0.0.1:6379> SADD myset "apple" "banana" "cherry"
(integer) 3
127.0.0.1:6379> SMEMBERS myset
1) "apple"
2) "banana"
3) "cherry"

# Hash operations
127.0.0.1:6379> HSET user:1 name "Alice" age "30"
(integer) 1
127.0.0.1:6379> HGETALL user:1
1) "name"
2) "Alice"
3) "age"
4) "30"

# Pub/Sub
127.0.0.1:6379> PUBLISH news "Breaking: RustRedis is awesome!"
(integer) 0
```

## 🏗️ Architecture Overview

RustRedis is built with a client-server model using Tokio's asynchronous runtime. The system follows clean architecture principles with clear separation of concerns.

### Core Components

**1. Server Layer (`src/bin/server.rs`)**
   - Asynchronous TCP listener bound to port 6379
   - Handles multiple concurrent client connections
   - Implements graceful shutdown on CTRL+C
   - AOF command logging and replay on startup
   - Command processing loop for each connection

**2. Protocol Layer (`src/frame.rs`, `src/connection.rs`)**
   - Complete RESP (Redis Serialization Protocol) parser
   - Supports all 6 RESP data types:
     - Simple Strings: `+OK\r\n`
     - Errors: `-Error message\r\n`
     - Integers: `:1000\r\n`
     - Bulk Strings: `$5\r\nhello\r\n`
     - Arrays: `*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n`
     - Null: `$-1\r\n`
   - Connection wrapper with buffered reading/writing
   - Zero-copy byte manipulation using `bytes::BytesMut`

**3. Command Layer (`src/cmd/mod.rs`)**
   - Modular command enum architecture
   - Support for 30+ Redis commands
   - Argument parsing and validation
   - Command execution with database interaction
   - Write command detection for AOF logging
   - Graceful error handling for unknown commands

**4. Storage Layer (`src/db.rs`)**
   - Thread-safe in-memory key-value store
   - Support for multiple data types:
     - **Strings**: Basic key-value pairs
     - **Lists**: VecDeque for efficient push/pop operations
     - **Sets**: HashSet for unique membership
     - **Hashes**: Nested HashMap for field-value pairs
   - Shared state using `Arc<Mutex<HashMap>>`
   - TTL (Time To Live) support with automatic expiration
   - Lazy expiration cleanup on key access
   - Pattern matching with glob support

**5. Persistence Layer (`src/persistence.rs`)**
   - AOF (Append-Only File) implementation
   - Three sync policies:
     - **Always**: Sync after every write (safest, slowest)
     - **EverySecond**: Sync every second (balanced, default)
     - **No**: Let OS decide (fastest, least safe)
   - Command replay on server startup
   - RESP serialization for persistence

**6. Pub/Sub Layer (`src/pubsub.rs`)**
   - Channel-based messaging system
   - Broadcast channels using Tokio
   - Dynamic channel creation
   - Automatic cleanup of empty channels
   - Support for multiple subscribers per channel

## 📦 Technology Stack

- **Rust 2021 Edition** - Systems programming language for safety and performance
- **Tokio** - Asynchronous runtime for concurrent I/O operations
- **Bytes** - Zero-copy byte buffer manipulation
- **Tracing** - Structured, async-aware logging framework
- **Anyhow** - Ergonomic error handling

## 💻 Implemented Commands

### String Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **SET** | `SET key value [EX seconds]` | Set key to hold the string value with optional expiration |
| **GET** | `GET key` | Get the value of a key. Returns `nil` if key doesn't exist |

### List Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **LPUSH** | `LPUSH key value [value ...]` | Insert values at the head of the list |
| **RPUSH** | `RPUSH key value [value ...]` | Insert values at the tail of the list |
| **LPOP** | `LPOP key` | Remove and return the first element of the list |
| **RPOP** | `RPOP key` | Remove and return the last element of the list |
| **LRANGE** | `LRANGE key start stop` | Get a range of elements from a list |
| **LLEN** | `LLEN key` | Get the length of a list |

### Set Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **SADD** | `SADD key member [member ...]` | Add members to a set |
| **SREM** | `SREM key member [member ...]` | Remove members from a set |
| **SMEMBERS** | `SMEMBERS key` | Get all members of a set |
| **SISMEMBER** | `SISMEMBER key member` | Check if member exists in a set |
| **SCARD** | `SCARD key` | Get the cardinality (size) of a set |

### Hash Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **HSET** | `HSET key field value` | Set a field in a hash |
| **HGET** | `HGET key field` | Get a field from a hash |
| **HGETALL** | `HGETALL key` | Get all fields and values from a hash |
| **HDEL** | `HDEL key field [field ...]` | Delete fields from a hash |
| **HEXISTS** | `HEXISTS key field` | Check if a field exists in a hash |
| **HLEN** | `HLEN key` | Get the number of fields in a hash |

### Utility Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **PING** | `PING [message]` | Test connection. Returns `PONG` or echoes message |
| **ECHO** | `ECHO message` | Echo the given string back to the client |
| **DEL** | `DEL key [key ...]` | Delete one or more keys |
| **EXISTS** | `EXISTS key` | Check if key exists |
| **TYPE** | `TYPE key` | Get the type of a value |
| **KEYS** | `KEYS pattern` | Get all keys matching a pattern |
| **DBSIZE** | `DBSIZE` | Get the number of keys in the database |
| **FLUSHDB** | `FLUSHDB` | Clear all keys from the database |

### Pub/Sub Commands
| Command | Syntax | Description |
|---------|--------|-------------|
| **PUBLISH** | `PUBLISH channel message` | Publish a message to a channel |

### Command Examples

```bash
# PING - Connection test
> PING
PONG
> PING "Hello World"
"Hello World"

# String operations
> SET name "RustRedis"
OK
> SET session "abc123" EX 3600
OK
> GET name
"RustRedis"

# List operations
> LPUSH tasks "task3" "task2" "task1"
(integer) 3
> LRANGE tasks 0 -1
1) "task1"
2) "task2"
3) "task3"
> LPOP tasks
"task1"
> LLEN tasks
(integer) 2

# Set operations
> SADD tags "rust" "redis" "async"
(integer) 3
> SISMEMBER tags "rust"
(integer) 1
> SCARD tags
(integer) 3
> SMEMBERS tags
1) "rust"
2) "redis"
3) "async"

# Hash operations
> HSET user:100 name "Alice" email "alice@example.com" age "30"
(integer) 1
> HGET user:100 name
"Alice"
> HGETALL user:100
1) "name"
2) "Alice"
3) "email"
4) "alice@example.com"
5) "age"
6) "30"
> HLEN user:100
(integer) 3

# Utility commands
> KEYS user:*
1) "user:100"
> TYPE user:100
hash
> EXISTS user:100
(integer) 1
> DBSIZE
(integer) 5
> DEL session
(integer) 1

# Pub/Sub
> PUBLISH news "Breaking news!"
(integer) 0

# ECHO - Echo messages
> ECHO "Testing RustRedis"
"Testing RustRedis"
```

## 🛠️ Building and Running

### Prerequisites
- Rust 1.70 or later
- Cargo (comes with Rust)
- Optional: redis-cli for testing

### Build

```bash
# Development build
cargo build

# Production build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin server
```

### Run Server

```bash
# Run in development mode
cargo run --bin server

# Run release build
./target/release/server
```

The server will start on `127.0.0.1:6379` and display:
```
INFO RustRedis server listening on 127.0.0.1:6379
INFO Press CTRL+C to shutdown gracefully
```

### Connect and Test

```bash
# Using redis-cli
redis-cli -p 6379

# Using telnet
telnet localhost 6379

# Using netcat
nc localhost 6379
```

## 📁 Project Structure

```
RustRedis/
├── src/
│   ├── bin/
│   │   └── server.rs          # Server entry point with main loop
│   ├── cmd/
│   │   └── mod.rs             # Command enum and execution logic (30+ commands)
│   ├── connection.rs          # Connection wrapper for frame I/O
│   ├── db.rs                  # Multi-type database with TTL support
│   ├── frame.rs               # RESP protocol parser
│   ├── persistence.rs         # AOF (Append-Only File) implementation
│   ├── pubsub.rs              # Pub/Sub messaging system
│   ├── lib.rs                 # Library root
│   └── main.rs                # Default binary (unused)
├── Cargo.toml                 # Dependencies and metadata
├── Cargo.lock                 # Dependency lock file
├── appendonly.aof             # AOF persistence file (created at runtime)
└── README.md                  # This file
```

## 🎯 Development Principles

This project follows industry best practices:

- **Clean Architecture** - Clear separation between protocol, domain, and infrastructure layers
- **Idiomatic Rust** - Following Rust best practices and ownership patterns
- **Async-First** - Non-blocking I/O for maximum concurrency
- **Type Safety** - Leveraging Rust's type system for correctness
- **Zero-Copy** - Efficient memory usage with shared references
- **Error Handling** - Proper error propagation with `Result` and `anyhow`
- **Logging** - Structured logging for production observability

## ⚡ Performance Features

- **Asynchronous I/O** - Handle thousands of concurrent connections
- **Zero-Copy Parsing** - Minimal memory allocations using `bytes::Bytes`
- **Lazy Expiration** - Keys expire only when accessed, reducing overhead
- **Buffered Writes** - Efficient batching of network writes
- **Lock Granularity** - Minimal lock contention with targeted mutex usage

## 🗺️ Roadmap

### Completed ✅
- [x] Basic TCP server with async I/O
- [x] RESP protocol implementation (all 6 data types)
- [x] Core commands (PING, SET, GET, ECHO)
- [x] TTL support with automatic expiration
- [x] Graceful shutdown handling
- [x] Structured logging with tracing
- [x] Thread-safe shared state
- [x] Zero-copy byte handling
- [x] Multiple data structures (Strings, Lists, Sets, Hashes)
- [x] 30+ Redis commands implemented
- [x] AOF (Append-Only File) persistence
- [x] Pub/Sub messaging (PUBLISH command)
- [x] Pattern matching with KEYS command
- [x] Database management (DBSIZE, FLUSHDB)
- [x] Utility commands (DEL, EXISTS, TYPE)

### Future Enhancements 📋
- [ ] SUBSCRIBE/UNSUBSCRIBE commands for Pub/Sub
- [ ] RDB snapshots for persistence
- [ ] Transactions (MULTI/EXEC/DISCARD/WATCH)
- [ ] Sorted Sets data structure
- [ ] Replication (master-slave)
- [ ] Lua scripting support
- [ ] Clustering support
- [ ] Memory eviction policies (LRU, LFU)
- [ ] Blocking list operations (BLPOP, BRPOP)
- [ ] Bit operations (SETBIT, GETBIT)
- [ ] HyperLogLog commands
- [ ] Geospatial indexes
- [ ] Streams data structure
- [ ] Comprehensive test coverage
- [ ] Benchmarking suite
- [ ] TLS/SSL support

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Check code quality
cargo clippy

# Format code
cargo fmt
```

## 📊 Code Quality

- ✅ **No Clippy Warnings** - Passes all linting checks
- ✅ **Formatted** - Code formatted with rustfmt
- ✅ **Documented** - Inline documentation for all public APIs
- ✅ **Type-Safe** - Leverages Rust's ownership system
- ✅ **Error Handling** - Proper `Result` types throughout

## 📝 License

MIT License - See LICENSE file for details

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure:
- Code passes `cargo clippy` with no warnings
- Code is formatted with `cargo fmt`
- Tests pass with `cargo test`
- New features include documentation

## 🙏 Acknowledgments

- Inspired by the [Redis](https://redis.io/) in-memory data store
- Built with [Tokio](https://tokio.rs/) async runtime
- Following patterns from the Rust community

## 📧 Contact

For questions or feedback, please open an issue on GitHub.

---

**Built with ❤️ in Rust**
