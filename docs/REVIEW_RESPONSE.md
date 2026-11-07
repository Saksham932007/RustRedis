# Response to the code review

An external review of the repository found parser crashes, missing tests and CI, and
two possible validity problems in the experiment. This page records what was found,
what was changed, and what is still open. It is written to be checked, not admired.

## What the review found, and what happened

| Finding | Status | Where it is handled |
|---|---|---|
| `*-5\r\n` panics (capacity overflow); `$-5\r\n` mis-parses | Fixed | `src/frame.rs`: lengths below -1 are `Invalid`; checked arithmetic |
| Unbounded array nesting overflows the stack (process-wide crash) | Fixed | `MAX_DEPTH = 128` in `src/frame.rs`; test nests 100,000 levels |
| No input size limit; unterminated input grows the buffer forever | Fixed | `MAX_BULK_LEN`, `MAX_ARRAY_LEN` in `src/frame.rs`; per-connection buffer cap in `src/connection.rs` |
| `SET k v EX 18446744073709551615` panics | Fixed | `checked_add` in `src/cmd/mod.rs`; `EX 0` also rejected |
| One poisoned mutex bricks the server | Fixed | `lock_recover()` in `src/sync.rs`, used everywhere a std `Mutex` is locked |
| Keys not binary-safe | Fixed | Keys, set members and hash fields are `Bytes`; `KEYS` matches on bytes |
| Comment says "zero-copy" but the parser copies | Fixed | Comment and `docs/system-design.md` corrected |
| `flush` after every frame | Changed | Responses are flushed once before the connection blocks on input |
| `db_dashmap.rs` never wired in | Fixed | `Store` trait; `RUSTREDIS_STORE=mutex\|dashmap`; `--store` in the runner |
| No tests for the parser, commands, or server | Fixed | Unit tests, plus `tests/server.rs` end-to-end on both stores |
| No CI | Fixed | `.github/workflows/ci.yml`: rustfmt, clippy `-D warnings`, tests |
| Parser never fuzzed | Done | `fuzz/`, see below |
| 15 MB of CSV in git | Fixed | Fetched from Zenodo by `scripts/fetch_data.sh` |
| README long before `cargo run` | Addressed | Full results kept for readers of the paper; a "Run it" section with the environment variables and a CI badge were added |
| Unknown strategy names silently defaulted | Fixed | `MetricsStrategy::parse` is strict in the server |
| Protocol and command errors silently dropped | Fixed | The server replies `-ERR ...` |

## Fuzzing

`fuzz/fuzz_targets/parse_frame.rs` feeds arbitrary bytes through `Frame::parse` and then
`Command::from_frame`. Run against the original commit it crashed within seconds, in
`skip` on length overflow. Run against the hardened parser it found no crashes in
a one-hour run (29,255,613 executions, no crashes, no timeouts).

## Answers to the review's interview questions

**1. What does `*-5\r\n` do?** Originally, `check_complete` treated the count as an empty
range and returned `Ok`; `parse_frame` then did `Vec::with_capacity(-5i64 as usize)`,
which is a capacity-overflow panic. The panic killed that connection's task. `$-5` was
also mishandled: the negative length wrapped in the length arithmetic. Both now return an
invalid-frame error and the client receives `-ERR Protocol error: ...`.

**2. The data path is one global mutex and the paper measures collector lock
contention. Why isn't that a confound?** It may be. I cannot rule it out from the
existing data, which were all measured on the single-mutex store. What I can say is
that the confound is now testable: `RUSTREDIS_STORE=dashmap` runs the same experiment
on a sharded keyspace. A quick local check
([CONFOUND_CHECK.md](CONFOUND_CHECK.md)) was flat across every store and collector,
which means the local setup is bounded by the load generator and loopback, so it does
not discriminate. The decisive experiment is the full RMIT design with
`--store dashmap` on the original hardware, which has not been run. Until then the
cardinality-before-contention finding stands as measured on the mutex store, with this
caveat attached.

**3. `db_dashmap.rs` has tests and isn't used. What happened?** I built it as the
sharded alternative and never connected it to the server. It is now behind the `Store`
trait and exercised by the end-to-end tests.

**4. Why does `write_frame` flush every frame?** It was the simplest way to make sure
responses were sent. It cost one `write` syscall per command, which defeats the
`BufWriter` and prevents batching under pipelining. Responses are now flushed when the
connection is about to wait for input. For the one-command-at-a-time client the
benchmark uses, that is still one flush per command, and the local before/after
throughput was unchanged ([CONFOUND_CHECK.md](CONFOUND_CHECK.md)). So the review is
right that the `disabled` baseline is plausibly syscall-bound, and the change does not
alter that for this workload. Overhead percentages are fractions of that baseline.

**5. Which finding is weakest?** The mechanism claimed for rigidity (the HDR-merge
explanation) is interpretation rather than measurement, as the paper already says. The
finding most exposed by this review is finding 1 (cardinality beats contention), for the
reason in answer 2.

## What is still open

- The `dashmap` re-run of the full design matrix on the original hardware.
- A pipelined benchmark, which is the only workload where the flush change matters.
- The parse path still copies each bulk payload and parses a frame twice (once to
  check it is complete, once to build it). Neither is a correctness problem.
