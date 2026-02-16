# Benchmark Results Summary

| Target | Workload | Concurrency | ops/sec | p50 (µs) | p95 (µs) | p99 (µs) | max (µs) |
|--------|----------|-------------|---------|----------|----------|----------|----------|
| Redis | Mixed (50% GET / 50% SET) | 1 | 32,912 | 23 | 48 | 124 | 3,747 |
| Redis | Mixed (50% GET / 50% SET) | 10 | 80,123 | 105 | 158 | 427 | 4,303 |
| Redis | Mixed (50% GET / 50% SET) | 100 | 94,300 | 883 | 1,553 | 3,282 | 7,420 |
| Redis | Mixed (50% GET / 50% SET) | 500 | 41,167 | 3,965 | 21,120 | 102,032 | 130,362 |
| Redis | Mixed (50% GET / 50% SET) | 1000 | 48,300 | 1,791 | 54,692 | 72,577 | 79,809 |
| Redis | Read-Heavy (80% GET / 20% SET) | 1 | 42,020 | 19 | 39 | 60 | 669 |
| Redis | Read-Heavy (80% GET / 20% SET) | 10 | 119,250 | 74 | 105 | 138 | 3,964 |
| Redis | Read-Heavy (80% GET / 20% SET) | 100 | 85,787 | 981 | 1,794 | 3,554 | 6,994 |
| Redis | Read-Heavy (80% GET / 20% SET) | 500 | 70,687 | 2,500 | 4,476 | 67,413 | 81,579 |
| Redis | Read-Heavy (80% GET / 20% SET) | 1000 | 8,954 | 2,006 | 61,545 | 95,651 | 101,934 |
| Redis | Write-Heavy (80% SET / 20% GET | 1 | 41,229 | 19 | 39 | 64 | 430 |
| Redis | Write-Heavy (80% SET / 20% GET | 10 | 115,277 | 75 | 116 | 193 | 3,030 |
| Redis | Write-Heavy (80% SET / 20% GET | 100 | 113,345 | 730 | 1,349 | 1,912 | 6,084 |
| Redis | Write-Heavy (80% SET / 20% GET | 500 | 72,010 | 2,185 | 6,649 | 59,268 | 70,223 |
| Redis | Write-Heavy (80% SET / 20% GET | 1000 | 8,998 | 1,749 | 56,536 | 77,292 | 79,394 |
| RustRedis | Mixed (50% GET / 50% SET) | 1 | 22,746 | 33 | 87 | 150 | 6,633 |
| RustRedis | Mixed (50% GET / 50% SET) | 10 | 40,139 | 105 | 520 | 2,358 | 13,172 |
| RustRedis | Mixed (50% GET / 50% SET) | 100 | 37,625 | 2,138 | 5,150 | 8,458 | 17,794 |
| RustRedis | Mixed (50% GET / 50% SET) | 500 | 48,909 | 1,135 | 3,361 | 5,688 | 13,553 |
| RustRedis | Mixed (50% GET / 50% SET) | 1000 | 42,093 | 536 | 3,228 | 10,688 | 14,762 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 1 | 34,827 | 23 | 59 | 90 | 4,767 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 10 | 81,032 | 99 | 186 | 459 | 12,724 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 100 | 74,018 | 1,028 | 2,264 | 4,615 | 14,802 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 500 | 64,844 | 1,705 | 5,299 | 6,932 | 23,023 |
| RustRedis | Read-Heavy (80% GET / 20% SET) | 1000 | 36,772 | 722 | 4,919 | 12,576 | 16,267 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 1 | 26,528 | 33 | 60 | 86 | 2,162 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 10 | 65,600 | 121 | 225 | 711 | 12,173 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 100 | 66,586 | 1,177 | 2,777 | 5,497 | 15,479 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 500 | 48,854 | 3,472 | 10,265 | 13,699 | 18,770 |
| RustRedis | Write-Heavy (80% SET / 20% GET | 1000 | 41,115 | 3,039 | 16,926 | 24,411 | 31,517 |
