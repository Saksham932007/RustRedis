# RustRedis

A high-performance Redis clone implemented in Rust, featuring full RESP (Redis Serialization Protocol) support and an asynchronous event-driven architecture.

![Rust](https://img.shields.io/badge/rust-2021-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)

## ✨ Features

- ✅ **Full RESP Protocol Support** - All 6 RESP data types implemented
- ✅ **Async I/O** - Built on Tokio for high-performance concurrent connections
- ✅ **TTL/Expiration** - Keys can expire automatically with lazy cleanup
- ✅ **Thread-Safe** - Shared state using `Arc<Mutex<HashMap>>`
- ✅ **Zero-Copy** - Efficient byte handling with the `bytes` crate
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
127.0.0.1:6379> SET mykey "Hello, RustRedis!"
OK
127.0.0.1:6379> GET mykey
"Hello, RustRedis!"
127.0.0.1:6379> SET tempkey "expires soon" EX 60
OK
127.0.0.1:6379> ECHO "It works!"
"It works!"
```

## 🏗️ Architecture Overview

RustRedis is built with a client-server model using Tokio's asynchronous runtime. The system follows clean architecture principles with clear separation of concerns.

### Core Components

**1. Server Layer (`src/bin/server.rs`)**
   - Asynchronous TCP listener bound to port 6379
   - Handles multiple concurrent client connections
   - Implements graceful shutdown on CTRL+C
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
   - Argument parsing and validation
   - Command execution with database interaction
   - Graceful error handling for unknown commands

**4. Storage Layer (`src/db.rs`)**
   - Thread-safe in-memory key-value store
   - Shared state using `Arc<Mutex<HashMap>>`
   - TTL (Time To Live) support with automatic expiration
   - Lazy expiration cleanup on key access

## 📦 Technology Stack

- **Rust 2021 Edition** - Systems programming language for safety and performance
- **Tokio** - Asynchronous runtime for concurrent I/O operations
- **Bytes** - Zero-copy byte buffer manipulation
- **Tracing** - Structured, async-aware logging framework
- **Anyhow** - Ergonomic error handling

## 💻 Implemented Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| **PING** | `PING [message]` | Test connection. Returns `PONG` or echoes message |
| **SET** | `SET key value [EX seconds]` | Set key to hold the string value with optional expiration |
| **GET** | `GET key` | Get the value of a key. Returns `nil` if key doesn't exist |
| **ECHO** | `ECHO message` | Echo the given string back to the client |

### Command Examples

```bash
# PING - Connection test
> PING
PONG
> PING "Hello World"
"Hello World"

# SET - Store values
> SET name "RustRedis"
OK
> SET session "abc123" EX 3600
OK

# GET - Retrieve values
> GET name
"RustRedis"
> GET nonexistent
(nil)

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
│   │   └── mod.rs             # Command enum and execution logic
│   ├── connection.rs          # Connection wrapper for frame I/O
│   ├── db.rs                  # Database with TTL support
│   ├── frame.rs               # RESP protocol parser
│   ├── lib.rs                 # Library root
│   └── main.rs                # Default binary (unused)
├── Cargo.toml                 # Dependencies and metadata
├── Cargo.lock                 # Dependency lock file
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

### Planned 📋
- [ ] Additional commands (DEL, EXISTS, KEYS, etc.)
- [ ] Persistence (RDB snapshots, AOF)
- [ ] Replication (master-slave)
- [ ] Pub/Sub messaging
- [ ] Transactions (MULTI/EXEC)
- [ ] Lua scripting support
- [ ] Clustering support
- [ ] Additional data structures (Lists, Sets, Hashes, Sorted Sets)
- [ ] Benchmarking suite
- [ ] Comprehensive test coverage

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
