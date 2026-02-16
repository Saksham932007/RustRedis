# Benchmark Results Summary

| Target | Workload | Concurrency | ops/sec | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) |
|--------|----------|-------------|---------|----------|----------|----------|----------|
| RustRedis | Mixed (50% GET / 50% SET) | 1 | 16,231 | 33 | 70 | 404 | 8,406 |
| RustRedis | Mixed (50% GET / 50% SET) | 10 | 66,790 | 99 | 221 | 837 | 13,832 |
| RustRedis | Mixed (50% GET / 50% SET) | 100 | 56,977 | 1,286 | 3,903 | 5,928 | 16,258 |
| RustRedis | Mixed (50% GET / 50% SET) | 500 | 45,421 | 2,643 | 8,731 | 12,587 | 16,024 |
| RustRedis | Mixed (50% GET / 50% SET) | 1000 | 33,721 | 1,814 | 8,741 | 16,337 | 25,211 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 1 | 28,120 | 27 | 67 | 126 | 5,483 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 10 | 72,652 | 82 | 271 | 1,398 | 8,476 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 100 | 76,476 | 867 | 2,578 | 7,288 | 13,410 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 500 | 56,663 | 1,436 | 7,669 | 14,367 | 42,167 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 1000 | 47,662 | 306 | 2,393 | 4,653 | 18,704 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 1 | 25,785 | 33 | 61 | 91 | 7,015 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 10 | 65,171 | 101 | 267 | 775 | 8,647 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 100 | 51,319 | 1,532 | 4,029 | 5,513 | 17,982 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 500 | 41,445 | 4,710 | 12,999 | 16,336 | 20,872 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 1000 | 23,856 | 6,605 | 18,582 | 24,796 | 33,953 |
