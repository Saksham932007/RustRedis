//! End-to-end tests against the real server binary, run on both keyspace stores.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server {
    child: Child,
    addr: String,
}

impl Server {
    fn start(store: &str) -> Server {
        // Ask the OS for a free port, then hand it to the server.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let addr = format!("127.0.0.1:{}", port);
        let child = Command::new(env!("CARGO_BIN_EXE_server"))
            .env("RUSTREDIS_BIND", &addr)
            .env("RUSTREDIS_STORE", store)
            .env("RUSTREDIS_DISABLE_AOF", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start server");

        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(&addr).is_err() {
            assert!(Instant::now() < deadline, "server did not start");
            std::thread::sleep(Duration::from_millis(50));
        }
        Server { child, addr }
    }

    fn connect(&self) -> Client {
        let stream = TcpStream::connect(&self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        Client { stream }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Client {
    stream: TcpStream,
}

impl Client {
    fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).unwrap();
    }

    /// Read whatever the server sends until it goes quiet.
    fn read_all(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = self.stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        out
    }

    fn cmd(&mut self, parts: &[&[u8]]) -> Vec<u8> {
        self.send_raw(&encode(parts));
        self.read_all()
    }
}

fn encode(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        out.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
    out
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn for_each_store(test: impl Fn(&Server)) {
    for store in ["mutex", "dashmap"] {
        let server = Server::start(store);
        test(&server);
    }
}

#[test]
fn ping_set_get_del() {
    for_each_store(|server| {
        let mut c = server.connect();
        assert_eq!(c.cmd(&[b"PING"]), b"+PONG\r\n");
        assert_eq!(c.cmd(&[b"SET", b"k", b"v"]), b"+OK\r\n");
        assert_eq!(c.cmd(&[b"GET", b"k"]), b"$1\r\nv\r\n");
        assert_eq!(c.cmd(&[b"DEL", b"k"]), b":1\r\n");
        assert_eq!(c.cmd(&[b"GET", b"k"]), b"$-1\r\n");
    });
}

#[test]
fn keys_expire() {
    for_each_store(|server| {
        let mut c = server.connect();
        assert_eq!(c.cmd(&[b"SET", b"k", b"v", b"EX", b"1"]), b"+OK\r\n");
        assert_eq!(c.cmd(&[b"GET", b"k"]), b"$1\r\nv\r\n");
        std::thread::sleep(Duration::from_millis(1300));
        assert_eq!(c.cmd(&[b"GET", b"k"]), b"$-1\r\n");
    });
}

#[test]
fn lists_sets_and_hashes() {
    for_each_store(|server| {
        let mut c = server.connect();
        assert_eq!(c.cmd(&[b"RPUSH", b"l", b"a", b"b"]), b":2\r\n");
        assert_eq!(c.cmd(&[b"LLEN", b"l"]), b":2\r\n");
        assert_eq!(c.cmd(&[b"LPOP", b"l"]), b"$1\r\na\r\n");

        assert_eq!(c.cmd(&[b"SADD", b"s", b"x", b"y"]), b":2\r\n");
        assert_eq!(c.cmd(&[b"SISMEMBER", b"s", b"x"]), b":1\r\n");
        assert_eq!(c.cmd(&[b"SCARD", b"s"]), b":2\r\n");

        assert_eq!(c.cmd(&[b"HSET", b"h", b"f", b"1"]), b":1\r\n");
        assert_eq!(c.cmd(&[b"HGET", b"h", b"f"]), b"$1\r\n1\r\n");
        assert_eq!(c.cmd(&[b"HLEN", b"h"]), b":1\r\n");
    });
}

#[test]
fn binary_keys_round_trip() {
    for_each_store(|server| {
        let mut c = server.connect();
        let key: &[u8] = b"\xff\xfe\x00key";
        assert_eq!(c.cmd(&[b"SET", key, b"v"]), b"+OK\r\n");
        assert_eq!(c.cmd(&[b"GET", key]), b"$1\r\nv\r\n");
        assert_eq!(c.cmd(&[b"SADD", b"s", b"\x80\x81"]), b":1\r\n");
        assert_eq!(c.cmd(&[b"SISMEMBER", b"s", b"\x80\x81"]), b":1\r\n");
    });
}

#[test]
fn pipelined_commands_all_get_replies() {
    for_each_store(|server| {
        let mut c = server.connect();
        let mut batch = Vec::new();
        for _ in 0..50 {
            batch.extend_from_slice(&encode(&[b"PING"]));
        }
        c.send_raw(&batch);
        assert_eq!(c.read_all(), b"+PONG\r\n".repeat(50));
    });
}

#[test]
fn malformed_input_gets_an_error_and_the_server_survives() {
    for_each_store(|server| {
        let attacks: Vec<Vec<u8>> = vec![
            b"*-5\r\n".to_vec(),
            b"$-5\r\n".to_vec(),
            b"*9223372036854775807\r\n".to_vec(),
            b"$600000000\r\n".to_vec(),
            // 100k nested arrays: used to overflow the stack and kill the process
            [b"*1\r\n".repeat(100_000), b":1\r\n".to_vec()].concat(),
            b"?garbage\r\n".to_vec(),
        ];
        for attack in &attacks {
            let mut c = server.connect();
            c.send_raw(attack);
            let reply = text(&c.read_all());
            assert!(reply.starts_with("-ERR"), "attack got {:?}", reply);
        }

        // The server is still up and serving.
        let mut c = server.connect();
        assert_eq!(c.cmd(&[b"PING"]), b"+PONG\r\n");
    });
}

#[test]
fn bad_commands_get_error_replies() {
    for_each_store(|server| {
        let mut c = server.connect();
        let reply = text(&c.cmd(&[b"SET", b"k", b"v", b"EX", b"18446744073709551615"]));
        assert!(reply.starts_with("-ERR"), "{}", reply);
        let reply = text(&c.cmd(&[b"SET", b"k", b"v", b"EX", b"0"]));
        assert!(reply.starts_with("-ERR"), "{}", reply);
        let reply = text(&c.cmd(&[b"NOSUCHCOMMAND"]));
        assert!(reply.starts_with("-ERR"), "{}", reply);
        // Same connection is still usable afterwards
        assert_eq!(c.cmd(&[b"PING"]), b"+PONG\r\n");
    });
}
