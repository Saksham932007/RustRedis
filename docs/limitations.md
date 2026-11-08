# Limitations

What RustRedis does not do, and what its measurements do not cover. The server exists to
host a controlled experiment on per-command metrics overhead, not to replace Redis.

## Server

- **Expiry is lazy only.** A key with a TTL is removed when a command next touches it
  and finds it expired. There is no background sweep, so an expired key that is never
  read again stays in memory until it is overwritten or deleted.
- **A single database.** There is one keyspace; `SELECT` and numbered databases are not
  implemented.
- **No clustering or replication.** One process, one node. No sharding across hosts, no
  replicas, no failover.
- **`KEYS` is a full scan.** On the default `mutex` store it walks every key while
  holding the global lock, so a large keyspace blocks every other command for the
  duration. On the `dashmap` store it iterates the shards without a single global lock,
  but it is still O(n) and returns a point-in-time-inconsistent view. Patterns are
  translated to a regex on each call.
- **Persistence is a simple append-only file.** There is no rewrite or compaction, so
  the file grows without bound. Durability under each fsync policy is described in
  [failure-analysis.md](failure-analysis.md).
- **Partial command set.** Strings, lists, sets, hashes and pub/sub, as listed in
  `src/cmd/mod.rs`. Anything else is an error.

## Measurement

- **Loopback, not network.** Client and server share one VM and talk over loopback. The
  results say nothing about network-bound deployments, and at the highest concurrency
  the setup is a thread-oversubscription stress test.
- **One VM family and image.** Replicated across size, instance, region and workload,
  but not across clouds or hardware families.
- **Only 4 and 8 workers.** The SYNC/CARD crossover near 11 workers is an extrapolation
  from two points, not a fitted law.
- **Default store only.** All published runs used the single-mutex keyspace store, which
  may compress synchronization-axis differences. See
  [CONFOUND_CHECK.md](CONFOUND_CHECK.md).
- **Mechanisms are interpretation.** The HDR-merge explanation for rigidity was not
  instrumented directly.

The measurement caveats are discussed in full in the README's Caveats section and in the
paper.
