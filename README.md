# RustRedis

A high-performance Redis clone implemented in Rust, featuring full RESP (Redis Serialization Protocol) support and an asynchronous event-driven architecture.

## Architecture Overview

RustRedis is built with a client-server model using Tokio's asynchronous runtime. The system follows clean architecture principles with clear separation of concerns.

### Core Components

1. **Server (`src/bin/server.rs`)**
   - Asynchronous TCP listener bound to port 6379
   - Handles multiple concurrent client connections
   - Implements graceful shutdown on CTRL+C

2. **Protocol Layer (`src/frame.rs`)**
   - RESP (Redis Serialization Protocol) parser
   - Supports all RESP data types:
     - Simple Strings (`+OK\r\n`)
     - Errors (`-Error message\r\n`)
     - Integers (`:1000\r\n`)
     - Bulk Strings (`$5\r\nhello\r\n`)
     - Arrays (`*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n`)
     - Null (`$-1\r\n`)

3. **Connection Management (`src/connection.rs`)**
   - Wraps TcpStream with buffered reading/writing
   - Frame parsing and serialization
   - Zero-copy byte manipulation using `bytes::BytesMut`

4. **Database Engine (`src/db.rs`)**
   - Thread-safe in-memory key-value store
   - Shared state using `Arc<Mutex<HashMap>>`
   - Support for TTL (Time To Live) with automatic expiration

5. **Command Processing (`src/cmd/`)**
   - Modular command architecture
   - Implemented commands:
     - `PING` - Connection test
     - `SET` - Store key-value pairs with optional TTL
     - `GET` - Retrieve values by key
     - `ECHO` - Echo back messages
   - Graceful handling of unknown commands

## Technology Stack

- **Rust 2021 Edition** - Systems programming language
- **Tokio** - Asynchronous runtime for concurrent I/O
- **Bytes** - Zero-copy byte buffer manipulation
- **Tracing** - Structured, async-aware logging
- **Anyhow** - Ergonomic error handling

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Cargo

### Build
```bash
cargo build --release
```

### Run Server
```bash
cargo run --bin server
```

The server will start on `127.0.0.1:6379`

### Connect with Redis CLI
```bash
redis-cli -p 6379
```

## Development Principles

- **Test-Driven Development (TDD)** - Tests written before implementation
- **Clean Architecture** - Clear separation between protocol, domain, and infrastructure layers
- **Idiomatic Rust** - Following Rust best practices and ownership patterns
- **Async-First** - Non-blocking I/O for maximum concurrency
- **Type Safety** - Leveraging Rust's type system for correctness

## Project Structure

```
rust-redis/
├── src/
│   ├── bin/
│   │   └── server.rs        # Server entry point
│   ├── cmd/
│   │   ├── mod.rs           # Command trait and parsing
│   │   ├── ping.rs          # PING command
│   │   ├── get.rs           # GET command
│   │   ├── set.rs           # SET command
│   │   ├── echo.rs          # ECHO command
│   │   └── unknown.rs       # Unknown command handler
│   ├── connection.rs        # Connection wrapper
│   ├── db.rs                # Database engine
│   ├── frame.rs             # RESP protocol implementation
│   └── lib.rs               # Library root
├── Cargo.toml
└── README.md
```

## Roadmap

- [x] Basic TCP server with async I/O
- [x] RESP protocol implementation
- [x] Core commands (PING, SET, GET, ECHO)
- [x] TTL support with automatic expiration
- [ ] Persistence (RDB snapshots)
- [ ] Replication (master-slave)
- [ ] Pub/Sub messaging
- [ ] Clustering support
- [ ] Additional data structures (Lists, Sets, Hashes)

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please follow the existing code style and ensure all tests pass before submitting pull requests.
