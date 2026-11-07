# Keyspace-store and flush-behaviour check

A reviewer raised two possible confounds in the metrics-collector comparison:

1. The request path runs through one global keyspace mutex (`Db`), so contention
   could be absorbed there before it ever reaches the collector.
2. `write_frame` flushed after every response, so the `disabled` baseline could be
   bound by write syscalls rather than by the store.

This note records what was done about each and what a quick local check can and
cannot say. It is not a replacement for re-running the paper's design.

## What changed in the code

- `RUSTREDIS_STORE=mutex|dashmap` selects the keyspace backend (`Db` or the sharded
  `DbDashMap`). The default stays `mutex`, so published results are reproducible.
- `scripts/run_rmit_experiment.py --store {mutex,dashmap}` passes it through and
  records it in `metadata_rmit.json`.
- Responses are now flushed once, just before the connection blocks on more input,
  instead of after every frame. A client that sends one command at a time still
  gets one flush per command; a pipelining client gets one per batch.

## Local smoke check

Setup: 4-core Linux machine, load generator on the same host, loopback, AOF off,
`rustredis-bench --requests 5000 --concurrency 50 --workload mixed --runs 3`.
One session, no randomised interleaving, so treat differences under about 2% as
noise.

| Server | Store | Collector | ops/s |
|---|---|---|---|
| flush per frame | mutex | disabled | 122,617 / 125,928 |
| flush batched | mutex | disabled | 124,521 / 126,300 |
| flush batched | dashmap | disabled | 124,439 / 124,532 |
| flush batched | mutex | global_mutex | 124,472 |
| flush batched | dashmap | global_mutex | 123,323 |
| flush batched | mutex | sharded_2key | 124,005 |
| flush batched | dashmap | sharded_2key | 122,088 |
| flush batched | mutex | thread_local | 124,126 |
| flush batched | dashmap | thread_local | 123,162 |

## What this does and does not show

- Throughput is about 124k ops/s in every cell. On this machine the load generator,
  loopback and per-request syscalls set the ceiling, not the keyspace lock and not
  the collector. So this check cannot separate the hypotheses: it is consistent
  with "contention is absorbed upstream" and with "contention never mattered here".
- The flush change makes no measurable difference for a one-command-at-a-time
  client, as expected. It only helps pipelined clients, which this benchmark does
  not exercise.
- Neither point answers the question for the paper's configuration. Doing that needs
  the full RMIT design on the original hardware with `--store dashmap`, compared
  against the existing `--store mutex` data, using the axis decomposition from the paper.
