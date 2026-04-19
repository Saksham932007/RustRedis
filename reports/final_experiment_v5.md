# final_experiment_v5

## Section 1 - Configuration Summary

- run_directory: results/final_experiment_v5/20260418_200904
- timestamp: 2026-04-18T20:09:04+05:30
- host: Sakshams-MacBook-Pro.local
- cpu: Apple M2
- runtime: rustc=rustc 1.94.1 (e408947bf 2026-03-25), cargo=cargo 1.94.1 (29ea6fb6a 2026-03-24)
- os: ProductName:		macOS;ProductVersion:		26.4;BuildVersion:		25E246;
- kernel: Darwin Sakshams-MacBook-Pro.local 25.4.0 Darwin Kernel Version 25.4.0: Thu Mar 19 19:33:09 PDT 2026; root:xnu-12377.101.15~1/RELEASE_ARM64_T8112 arm64
- logical_cpu: 8
- physical_cpu: 8
- memory_bytes: 8589934592
- metrics_strategies: Disabled, GlobalMutex, Sharded, ThreadLocal
- sharded_shard_count: 64
- threadlocal_flush_config: 1000_records_or_100ms
- workload: mixed (50% GET / 50% SET)
- runs_per_configuration: 30
- concurrency_levels: 100, 500, 1000
- request_matrix:
| Concurrency | Requests per Client | Total Requests |
|---:|---:|---:|
| 100 | 1000 | 100000 |
| 500 | 1000 | 500000 |
| 1000 | 1000 | 1000000 |
- server_restart_per_configuration: true
- wait_before_benchmark_seconds: 3
- wait_between_runs_seconds: 3
- wait_between_strategies_seconds: 5
- reproducible_runner_command: ./benchmarks/run_final_experiment_v5.sh

## Section 2 - Raw Data (FULL)

### Strategy: Disabled | Concurrency: 100 | Requests per Client: 1000 | Total Requests: 100000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 30457.241621 | 40322.000000 | 0 | - |
| 2 | SUCCESS | 39757.898254 | 28935.000000 | 0 | - |
| 3 | SUCCESS | 41424.027583 | 26943.000000 | 0 | - |
| 4 | SUCCESS | 33902.687059 | 27581.000000 | 0 | - |
| 5 | SUCCESS | 30182.700419 | 31969.000000 | 0 | - |
| 6 | SUCCESS | 47883.502320 | 23176.000000 | 0 | - |
| 7 | SUCCESS | 34401.999441 | 26350.000000 | 0 | - |
| 8 | SUCCESS | 33139.212575 | 30934.000000 | 0 | - |
| 9 | SUCCESS | 49377.198190 | 18726.000000 | 0 | - |
| 10 | SUCCESS | 27079.518295 | 31204.000000 | 0 | - |
| 11 | SUCCESS | 36361.730678 | 35810.000000 | 0 | - |
| 12 | SUCCESS | 41749.522787 | 28363.000000 | 0 | - |
| 13 | SUCCESS | 36708.376972 | 25778.000000 | 0 | - |
| 14 | SUCCESS | 40523.140222 | 20619.000000 | 0 | - |
| 15 | SUCCESS | 31418.280068 | 31115.000000 | 0 | - |
| 16 | SUCCESS | 41065.933369 | 29273.000000 | 0 | - |
| 17 | SUCCESS | 38057.484462 | 23107.000000 | 0 | - |
| 18 | SUCCESS | 35077.568342 | 28285.000000 | 0 | - |
| 19 | SUCCESS | 49214.465378 | 20451.000000 | 0 | - |
| 20 | SUCCESS | 26818.712374 | 34758.000000 | 0 | - |
| 21 | SUCCESS | 32506.812856 | 31025.000000 | 0 | - |
| 22 | SUCCESS | 29538.778194 | 36221.000000 | 0 | - |
| 23 | SUCCESS | 34970.966383 | 30977.000000 | 0 | - |
| 24 | SUCCESS | 25993.755626 | 30635.000000 | 0 | - |
| 25 | SUCCESS | 33373.572127 | 29533.000000 | 0 | - |
| 26 | SUCCESS | 26023.243985 | 39884.000000 | 0 | - |
| 27 | SUCCESS | 60714.650893 | 13736.000000 | 0 | - |
| 28 | SUCCESS | 39013.432866 | 31897.000000 | 0 | - |
| 29 | SUCCESS | 22036.586425 | 32000.000000 | 0 | - |
| 30 | SUCCESS | 49572.859570 | 21836.000000 | 0 | - |

### Strategy: Disabled | Concurrency: 500 | Requests per Client: 1000 | Total Requests: 500000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 146295.196638 | 8938.000000 | 0 | - |
| 2 | SUCCESS | 142335.635256 | 9113.000000 | 0 | - |
| 3 | SUCCESS | 153125.539169 | 8371.000000 | 0 | - |
| 4 | SUCCESS | 147289.864620 | 9070.000000 | 0 | - |
| 5 | SUCCESS | 139756.929856 | 9433.000000 | 0 | - |
| 6 | SUCCESS | 146704.347338 | 8868.000000 | 0 | - |
| 7 | SUCCESS | 147736.899704 | 8818.000000 | 0 | - |
| 8 | SUCCESS | 146983.820323 | 8758.000000 | 0 | - |
| 9 | SUCCESS | 147588.035156 | 8764.000000 | 0 | - |
| 10 | SUCCESS | 146880.590413 | 8746.000000 | 0 | - |
| 11 | SUCCESS | 146075.381763 | 8742.000000 | 0 | - |
| 12 | SUCCESS | 144836.820775 | 9136.000000 | 0 | - |
| 13 | SUCCESS | 137925.016500 | 10744.000000 | 0 | - |
| 14 | SUCCESS | 147891.415924 | 8757.000000 | 0 | - |
| 15 | SUCCESS | 149427.215627 | 8703.000000 | 0 | - |
| 16 | SUCCESS | 150270.505724 | 8836.000000 | 0 | - |
| 17 | SUCCESS | 144188.056999 | 9055.000000 | 0 | - |
| 18 | SUCCESS | 151164.573751 | 8529.000000 | 0 | - |
| 19 | SUCCESS | 146529.401639 | 8898.000000 | 0 | - |
| 20 | SUCCESS | 150377.108178 | 8732.000000 | 0 | - |
| 21 | SUCCESS | 148509.728988 | 8714.000000 | 0 | - |
| 22 | SUCCESS | 143621.273029 | 9202.000000 | 0 | - |
| 23 | SUCCESS | 151045.575264 | 8560.000000 | 0 | - |
| 24 | SUCCESS | 152129.919677 | 8524.000000 | 0 | - |
| 25 | SUCCESS | 148476.104367 | 8594.000000 | 0 | - |
| 26 | SUCCESS | 153873.726834 | 8595.000000 | 0 | - |
| 27 | SUCCESS | 144028.087754 | 9177.000000 | 0 | - |
| 28 | SUCCESS | 147453.674111 | 8786.000000 | 0 | - |
| 29 | SUCCESS | 141213.342209 | 9280.000000 | 0 | - |
| 30 | SUCCESS | 150585.510316 | 8563.000000 | 0 | - |

### Strategy: Disabled | Concurrency: 1000 | Requests per Client: 1000 | Total Requests: 1000000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 148265.743003 | 17211.000000 | 0 | - |
| 2 | SUCCESS | 148443.575604 | 17139.000000 | 0 | - |
| 3 | SUCCESS | 148466.910278 | 17099.000000 | 0 | - |
| 4 | SUCCESS | 64600.198898 | 181101.000000 | 0 | - |
| 5 | SUCCESS | 32235.364364 | 279984.000000 | 0 | - |
| 6 | SUCCESS | 31566.369458 | 265679.000000 | 0 | - |
| 7 | SUCCESS | 32528.378975 | 262084.000000 | 0 | - |
| 8 | SUCCESS | 33871.115955 | 255090.000000 | 0 | - |
| 9 | SUCCESS | 36969.117580 | 254097.000000 | 0 | - |
| 10 | SUCCESS | 33019.555831 | 254720.000000 | 0 | - |
| 11 | SUCCESS | 31259.279570 | 274888.000000 | 0 | - |
| 12 | SUCCESS | 36017.733295 | 261399.000000 | 0 | - |
| 13 | SUCCESS | 34135.798344 | 239877.000000 | 0 | - |
| 14 | SUCCESS | 35382.076448 | 252893.000000 | 0 | - |
| 15 | SUCCESS | 35832.009757 | 251778.000000 | 0 | - |
| 16 | SUCCESS | 34569.842240 | 253873.000000 | 0 | - |
| 17 | SUCCESS | 35377.019530 | 250286.000000 | 0 | - |
| 18 | SUCCESS | 35271.549146 | 256135.000000 | 0 | - |
| 19 | SUCCESS | 33874.367103 | 248643.000000 | 0 | - |
| 20 | SUCCESS | 36054.114792 | 250870.000000 | 0 | - |
| 21 | SUCCESS | 37963.110922 | 238212.000000 | 0 | - |
| 22 | SUCCESS | 35567.070487 | 247033.000000 | 0 | - |
| 23 | SUCCESS | 35898.182327 | 243627.000000 | 0 | - |
| 24 | SUCCESS | 32912.693883 | 255975.000000 | 0 | - |
| 25 | SUCCESS | 35209.194416 | 244761.000000 | 0 | - |
| 26 | SUCCESS | 35704.124171 | 244359.000000 | 0 | - |
| 27 | SUCCESS | 33864.822246 | 256652.000000 | 0 | - |
| 28 | SUCCESS | 36859.624667 | 242865.000000 | 0 | - |
| 29 | SUCCESS | 34409.465667 | 242888.000000 | 0 | - |
| 30 | SUCCESS | 36612.635850 | 248683.000000 | 0 | - |

### Strategy: GlobalMutex | Concurrency: 100 | Requests per Client: 1000 | Total Requests: 100000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 156282.813286 | 1736.000000 | 0 | - |
| 2 | SUCCESS | 19220.851393 | 31906.000000 | 0 | - |
| 3 | SUCCESS | 19702.362971 | 27872.000000 | 0 | - |
| 4 | SUCCESS | 21138.088049 | 30735.000000 | 0 | - |
| 5 | SUCCESS | 21009.559770 | 27889.000000 | 0 | - |
| 6 | SUCCESS | 22925.467300 | 26710.000000 | 0 | - |
| 7 | SUCCESS | 20172.128437 | 29482.000000 | 0 | - |
| 8 | SUCCESS | 19763.784891 | 30752.000000 | 0 | - |
| 9 | SUCCESS | 20181.748080 | 28936.000000 | 0 | - |
| 10 | SUCCESS | 22079.447521 | 25543.000000 | 0 | - |
| 11 | SUCCESS | 19412.669995 | 31607.000000 | 0 | - |
| 12 | SUCCESS | 20103.402353 | 29283.000000 | 0 | - |
| 13 | SUCCESS | 19950.400475 | 26391.000000 | 0 | - |
| 14 | SUCCESS | 19775.948721 | 27118.000000 | 0 | - |
| 15 | SUCCESS | 22747.657084 | 30464.000000 | 0 | - |
| 16 | SUCCESS | 22007.478344 | 31255.000000 | 0 | - |
| 17 | SUCCESS | 20740.772055 | 27099.000000 | 0 | - |
| 18 | SUCCESS | 21739.331680 | 27789.000000 | 0 | - |
| 19 | SUCCESS | 23287.803543 | 28763.000000 | 0 | - |
| 20 | SUCCESS | 26798.118886 | 27837.000000 | 0 | - |
| 21 | SUCCESS | 58398.052367 | 16550.000000 | 0 | - |
| 22 | SUCCESS | 21111.695810 | 27990.000000 | 0 | - |
| 23 | SUCCESS | 21084.100522 | 27051.000000 | 0 | - |
| 24 | SUCCESS | 20227.256090 | 30591.000000 | 0 | - |
| 25 | SUCCESS | 18863.340676 | 29156.000000 | 0 | - |
| 26 | SUCCESS | 22176.131798 | 31443.000000 | 0 | - |
| 27 | SUCCESS | 19641.888754 | 29807.000000 | 0 | - |
| 28 | SUCCESS | 28504.818235 | 24992.000000 | 0 | - |
| 29 | SUCCESS | 44940.895714 | 19898.000000 | 0 | - |
| 30 | SUCCESS | 18953.137958 | 27220.000000 | 0 | - |

### Strategy: GlobalMutex | Concurrency: 500 | Requests per Client: 1000 | Total Requests: 500000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 48609.389474 | 120145.000000 | 0 | - |
| 2 | SUCCESS | 31793.335603 | 139974.000000 | 0 | - |
| 3 | SUCCESS | 31828.650249 | 136943.000000 | 0 | - |
| 4 | SUCCESS | 29969.397947 | 137228.000000 | 0 | - |
| 5 | SUCCESS | 28845.821218 | 135001.000000 | 0 | - |
| 6 | SUCCESS | 29332.132137 | 139717.000000 | 0 | - |
| 7 | SUCCESS | 30041.004395 | 135090.000000 | 0 | - |
| 8 | SUCCESS | 30205.200236 | 133842.000000 | 0 | - |
| 9 | SUCCESS | 31830.510256 | 131883.000000 | 0 | - |
| 10 | SUCCESS | 30656.323295 | 134578.000000 | 0 | - |
| 11 | SUCCESS | 29458.967140 | 130054.000000 | 0 | - |
| 12 | SUCCESS | 28937.062760 | 121679.000000 | 0 | - |
| 13 | SUCCESS | 30167.646590 | 133888.000000 | 0 | - |
| 14 | SUCCESS | 33498.754837 | 125800.000000 | 0 | - |
| 15 | SUCCESS | 35330.194035 | 128217.000000 | 0 | - |
| 16 | SUCCESS | 32479.975304 | 140068.000000 | 0 | - |
| 17 | SUCCESS | 31040.278702 | 128753.000000 | 0 | - |
| 18 | SUCCESS | 31471.574804 | 126340.000000 | 0 | - |
| 19 | SUCCESS | 36284.603657 | 133418.000000 | 0 | - |
| 20 | SUCCESS | 31119.180557 | 135308.000000 | 0 | - |
| 21 | SUCCESS | 29804.183460 | 133881.000000 | 0 | - |
| 22 | SUCCESS | 29225.419277 | 134706.000000 | 0 | - |
| 23 | SUCCESS | 29765.853684 | 140653.000000 | 0 | - |
| 24 | SUCCESS | 33395.952229 | 132102.000000 | 0 | - |
| 25 | SUCCESS | 33752.198112 | 133305.000000 | 0 | - |
| 26 | SUCCESS | 29829.574247 | 138008.000000 | 0 | - |
| 27 | SUCCESS | 31021.536562 | 137324.000000 | 0 | - |
| 28 | SUCCESS | 34577.587655 | 129306.000000 | 0 | - |
| 29 | SUCCESS | 31264.950753 | 126614.000000 | 0 | - |
| 30 | SUCCESS | 32118.280579 | 135597.000000 | 0 | - |

### Strategy: GlobalMutex | Concurrency: 1000 | Requests per Client: 1000 | Total Requests: 1000000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 40135.093588 | 247108.000000 | 0 | - |
| 2 | SUCCESS | 33574.647568 | 279054.000000 | 0 | - |
| 3 | SUCCESS | 30965.417732 | 267742.000000 | 0 | - |
| 4 | SUCCESS | 30892.271717 | 259672.000000 | 0 | - |
| 5 | SUCCESS | 31634.830296 | 255063.000000 | 0 | - |
| 6 | SUCCESS | 31498.722774 | 263841.000000 | 0 | - |
| 7 | SUCCESS | 33465.693265 | 261221.000000 | 0 | - |
| 8 | SUCCESS | 30678.997936 | 258566.000000 | 0 | - |
| 9 | SUCCESS | 31702.834202 | 254127.000000 | 0 | - |
| 10 | SUCCESS | 31485.488083 | 263183.000000 | 0 | - |
| 11 | SUCCESS | 30226.403699 | 248993.000000 | 0 | - |
| 12 | SUCCESS | 30655.291810 | 241692.000000 | 0 | - |
| 13 | SUCCESS | 30887.121686 | 268754.000000 | 0 | - |
| 14 | SUCCESS | 29630.472526 | 257968.000000 | 0 | - |
| 15 | SUCCESS | 31413.018925 | 267706.000000 | 0 | - |
| 16 | SUCCESS | 31599.894190 | 271338.000000 | 0 | - |
| 17 | SUCCESS | 35058.391416 | 252049.000000 | 0 | - |
| 18 | SUCCESS | 31984.196651 | 262192.000000 | 0 | - |
| 19 | SUCCESS | 30533.184552 | 265969.000000 | 0 | - |
| 20 | SUCCESS | 31334.467934 | 267904.000000 | 0 | - |
| 21 | SUCCESS | 31027.347432 | 267498.000000 | 0 | - |
| 22 | SUCCESS | 33187.817706 | 251934.000000 | 0 | - |
| 23 | SUCCESS | 32507.876493 | 255324.000000 | 0 | - |
| 24 | SUCCESS | 30605.007711 | 272745.000000 | 0 | - |
| 25 | SUCCESS | 31315.081751 | 264186.000000 | 0 | - |
| 26 | SUCCESS | 34132.445320 | 249775.000000 | 0 | - |
| 27 | SUCCESS | 32066.929368 | 255302.000000 | 0 | - |
| 28 | SUCCESS | 31542.546896 | 257978.000000 | 0 | - |
| 29 | SUCCESS | 31848.174572 | 261432.000000 | 0 | - |
| 30 | SUCCESS | 33563.331001 | 255541.000000 | 0 | - |

### Strategy: Sharded | Concurrency: 100 | Requests per Client: 1000 | Total Requests: 100000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 150321.980362 | 1807.000000 | 0 | - |
| 2 | SUCCESS | 20655.519967 | 27230.000000 | 0 | - |
| 3 | SUCCESS | 31324.698805 | 26652.000000 | 0 | - |
| 4 | SUCCESS | 21260.546104 | 34713.000000 | 0 | - |
| 5 | SUCCESS | 60579.882754 | 15739.000000 | 0 | - |
| 6 | SUCCESS | 21459.503770 | 27254.000000 | 0 | - |
| 7 | SUCCESS | 70718.225793 | 13649.000000 | 0 | - |
| 8 | SUCCESS | 20717.912080 | 32470.000000 | 0 | - |
| 9 | SUCCESS | 84222.861295 | 11032.000000 | 0 | - |
| 10 | SUCCESS | 22896.315617 | 30915.000000 | 0 | - |
| 11 | SUCCESS | 51851.600107 | 17366.000000 | 0 | - |
| 12 | SUCCESS | 20812.657983 | 30812.000000 | 0 | - |
| 13 | SUCCESS | 78893.383589 | 12079.000000 | 0 | - |
| 14 | SUCCESS | 21369.676600 | 28132.000000 | 0 | - |
| 15 | SUCCESS | 86054.721240 | 10267.000000 | 0 | - |
| 16 | SUCCESS | 20736.427614 | 28724.000000 | 0 | - |
| 17 | SUCCESS | 64892.089727 | 14213.000000 | 0 | - |
| 18 | SUCCESS | 23016.729459 | 27213.000000 | 0 | - |
| 19 | SUCCESS | 46889.969989 | 22098.000000 | 0 | - |
| 20 | SUCCESS | 20991.747041 | 26099.000000 | 0 | - |
| 21 | SUCCESS | 63491.541128 | 13940.000000 | 0 | - |
| 22 | SUCCESS | 19685.107342 | 32180.000000 | 0 | - |
| 23 | SUCCESS | 70431.946804 | 13916.000000 | 0 | - |
| 24 | SUCCESS | 13397.776341 | 33362.000000 | 0 | - |
| 25 | SUCCESS | 16938.802171 | 33185.000000 | 0 | - |
| 26 | SUCCESS | 54794.927111 | 19290.000000 | 0 | - |
| 27 | SUCCESS | 20988.913656 | 28378.000000 | 0 | - |
| 28 | SUCCESS | 52950.863373 | 20161.000000 | 0 | - |
| 29 | SUCCESS | 20627.341043 | 29919.000000 | 0 | - |
| 30 | SUCCESS | 46431.779651 | 23376.000000 | 0 | - |

### Strategy: Sharded | Concurrency: 500 | Requests per Client: 1000 | Total Requests: 500000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 151556.557823 | 8394.000000 | 0 | - |
| 2 | SUCCESS | 33233.815320 | 131771.000000 | 0 | - |
| 3 | SUCCESS | 31835.889979 | 129535.000000 | 0 | - |
| 4 | SUCCESS | 29454.928015 | 134917.000000 | 0 | - |
| 5 | SUCCESS | 32015.436649 | 136285.000000 | 0 | - |
| 6 | SUCCESS | 31506.299977 | 132709.000000 | 0 | - |
| 7 | SUCCESS | 31105.762902 | 124983.000000 | 0 | - |
| 8 | SUCCESS | 32976.701210 | 130552.000000 | 0 | - |
| 9 | SUCCESS | 31914.446984 | 136169.000000 | 0 | - |
| 10 | SUCCESS | 30661.570918 | 132337.000000 | 0 | - |
| 11 | SUCCESS | 32093.933465 | 137445.000000 | 0 | - |
| 12 | SUCCESS | 32034.695455 | 142254.000000 | 0 | - |
| 13 | SUCCESS | 31717.832998 | 136536.000000 | 0 | - |
| 14 | SUCCESS | 33140.659614 | 129366.000000 | 0 | - |
| 15 | SUCCESS | 31198.442219 | 130972.000000 | 0 | - |
| 16 | SUCCESS | 30369.994222 | 132077.000000 | 0 | - |
| 17 | SUCCESS | 34848.927822 | 123857.000000 | 0 | - |
| 18 | SUCCESS | 33294.736874 | 129462.000000 | 0 | - |
| 19 | SUCCESS | 33121.283699 | 128049.000000 | 0 | - |
| 20 | SUCCESS | 30953.634826 | 127768.000000 | 0 | - |
| 21 | SUCCESS | 30554.842792 | 134416.000000 | 0 | - |
| 22 | SUCCESS | 31722.006775 | 137211.000000 | 0 | - |
| 23 | SUCCESS | 28357.622697 | 120066.000000 | 0 | - |
| 24 | SUCCESS | 31144.616432 | 120089.000000 | 0 | - |
| 25 | SUCCESS | 32178.622158 | 132317.000000 | 0 | - |
| 26 | SUCCESS | 30982.502079 | 135152.000000 | 0 | - |
| 27 | SUCCESS | 31846.068505 | 129286.000000 | 0 | - |
| 28 | SUCCESS | 30063.137776 | 125577.000000 | 0 | - |
| 29 | SUCCESS | 31572.822241 | 135669.000000 | 0 | - |
| 30 | SUCCESS | 31956.916877 | 138012.000000 | 0 | - |

### Strategy: Sharded | Concurrency: 1000 | Requests per Client: 1000 | Total Requests: 1000000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 37742.404427 | 227875.000000 | 0 | - |
| 2 | SUCCESS | 31946.859083 | 266804.000000 | 0 | - |
| 3 | SUCCESS | 30781.751602 | 258640.000000 | 0 | - |
| 4 | SUCCESS | 31525.460111 | 263856.000000 | 0 | - |
| 5 | SUCCESS | 31011.807112 | 266957.000000 | 0 | - |
| 6 | SUCCESS | 31619.665052 | 261436.000000 | 0 | - |
| 7 | SUCCESS | 33178.389334 | 254676.000000 | 0 | - |
| 8 | SUCCESS | 32818.086504 | 266216.000000 | 0 | - |
| 9 | SUCCESS | 32463.033887 | 253420.000000 | 0 | - |
| 10 | SUCCESS | 32508.338657 | 249962.000000 | 0 | - |
| 11 | SUCCESS | 31897.551741 | 256557.000000 | 0 | - |
| 12 | SUCCESS | 32253.174475 | 259890.000000 | 0 | - |
| 13 | SUCCESS | 34042.716789 | 252813.000000 | 0 | - |
| 14 | SUCCESS | 32075.391368 | 256499.000000 | 0 | - |
| 15 | SUCCESS | 31634.786180 | 266692.000000 | 0 | - |
| 16 | SUCCESS | 31205.286522 | 252002.000000 | 0 | - |
| 17 | SUCCESS | 33057.306841 | 254091.000000 | 0 | - |
| 18 | SUCCESS | 30907.969233 | 235283.000000 | 0 | - |
| 19 | SUCCESS | 31107.841014 | 241376.000000 | 0 | - |
| 20 | SUCCESS | 31253.183999 | 240962.000000 | 0 | - |
| 21 | SUCCESS | 30804.858404 | 245801.000000 | 0 | - |
| 22 | SUCCESS | 31276.740351 | 255809.000000 | 0 | - |
| 23 | SUCCESS | 31909.055320 | 253669.000000 | 0 | - |
| 24 | SUCCESS | 31987.548740 | 263376.000000 | 0 | - |
| 25 | SUCCESS | 32208.969086 | 252796.000000 | 0 | - |
| 26 | SUCCESS | 29896.386114 | 258732.000000 | 0 | - |
| 27 | SUCCESS | 29782.724896 | 263918.000000 | 0 | - |
| 28 | SUCCESS | 32673.804022 | 250131.000000 | 0 | - |
| 29 | SUCCESS | 30896.306883 | 272873.000000 | 0 | - |
| 30 | SUCCESS | 30405.877377 | 264363.000000 | 0 | - |

### Strategy: ThreadLocal | Concurrency: 100 | Requests per Client: 1000 | Total Requests: 100000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 151887.832707 | 1770.000000 | 0 | - |
| 2 | SUCCESS | 147949.138365 | 1729.000000 | 0 | - |
| 3 | SUCCESS | 19709.275700 | 28604.000000 | 0 | - |
| 4 | SUCCESS | 28831.029338 | 28078.000000 | 0 | - |
| 5 | SUCCESS | 21255.121853 | 27975.000000 | 0 | - |
| 6 | SUCCESS | 19759.236501 | 28151.000000 | 0 | - |
| 7 | SUCCESS | 23221.148289 | 28997.000000 | 0 | - |
| 8 | SUCCESS | 20166.161609 | 30094.000000 | 0 | - |
| 9 | SUCCESS | 49638.981890 | 21795.000000 | 0 | - |
| 10 | SUCCESS | 22724.793876 | 35840.000000 | 0 | - |
| 11 | SUCCESS | 24258.186424 | 32607.000000 | 0 | - |
| 12 | SUCCESS | 20241.876420 | 26763.000000 | 0 | - |
| 13 | SUCCESS | 20283.934515 | 30377.000000 | 0 | - |
| 14 | SUCCESS | 24673.979879 | 31835.000000 | 0 | - |
| 15 | SUCCESS | 21510.544273 | 29015.000000 | 0 | - |
| 16 | SUCCESS | 79999.157321 | 11170.000000 | 0 | - |
| 17 | SUCCESS | 22245.570387 | 30322.000000 | 0 | - |
| 18 | SUCCESS | 21604.974113 | 30134.000000 | 0 | - |
| 19 | SUCCESS | 20766.053302 | 29303.000000 | 0 | - |
| 20 | SUCCESS | 19408.563335 | 27779.000000 | 0 | - |
| 21 | SUCCESS | 22206.906035 | 27554.000000 | 0 | - |
| 22 | SUCCESS | 25707.318659 | 28628.000000 | 0 | - |
| 23 | SUCCESS | 100945.688688 | 5746.000000 | 0 | - |
| 24 | SUCCESS | 20265.553736 | 29057.000000 | 0 | - |
| 25 | SUCCESS | 21312.183881 | 29531.000000 | 0 | - |
| 26 | SUCCESS | 20547.128072 | 26433.000000 | 0 | - |
| 27 | SUCCESS | 19867.131111 | 27870.000000 | 0 | - |
| 28 | SUCCESS | 23747.075593 | 31767.000000 | 0 | - |
| 29 | SUCCESS | 18441.316848 | 34150.000000 | 0 | - |
| 30 | SUCCESS | 52597.803296 | 21450.000000 | 0 | - |

### Strategy: ThreadLocal | Concurrency: 500 | Requests per Client: 1000 | Total Requests: 500000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 151035.967026 | 8615.000000 | 0 | - |
| 2 | SUCCESS | 148343.228680 | 8621.000000 | 0 | - |
| 3 | SUCCESS | 151006.587229 | 8446.000000 | 0 | - |
| 4 | SUCCESS | 150368.681452 | 8400.000000 | 0 | - |
| 5 | SUCCESS | 147796.161539 | 8775.000000 | 0 | - |
| 6 | SUCCESS | 149446.811449 | 8426.000000 | 0 | - |
| 7 | SUCCESS | 147562.792042 | 8724.000000 | 0 | - |
| 8 | SUCCESS | 147878.891752 | 8772.000000 | 0 | - |
| 9 | SUCCESS | 150022.602780 | 8562.000000 | 0 | - |
| 10 | SUCCESS | 148320.106084 | 8803.000000 | 0 | - |
| 11 | SUCCESS | 149147.454521 | 8444.000000 | 0 | - |
| 12 | SUCCESS | 149043.665064 | 8454.000000 | 0 | - |
| 13 | SUCCESS | 148913.122181 | 8641.000000 | 0 | - |
| 14 | SUCCESS | 149269.725884 | 8687.000000 | 0 | - |
| 15 | SUCCESS | 151028.173344 | 8453.000000 | 0 | - |
| 16 | SUCCESS | 148785.056594 | 8513.000000 | 0 | - |
| 17 | SUCCESS | 148787.487984 | 8476.000000 | 0 | - |
| 18 | SUCCESS | 148584.290765 | 8644.000000 | 0 | - |
| 19 | SUCCESS | 148157.273539 | 8651.000000 | 0 | - |
| 20 | SUCCESS | 146870.832449 | 8736.000000 | 0 | - |
| 21 | SUCCESS | 149461.730407 | 8486.000000 | 0 | - |
| 22 | SUCCESS | 150032.860947 | 8586.000000 | 0 | - |
| 23 | SUCCESS | 148586.739530 | 8866.000000 | 0 | - |
| 24 | SUCCESS | 151574.305685 | 8445.000000 | 0 | - |
| 25 | SUCCESS | 149776.737179 | 8375.000000 | 0 | - |
| 26 | SUCCESS | 147649.570154 | 8606.000000 | 0 | - |
| 27 | SUCCESS | 147211.669292 | 8667.000000 | 0 | - |
| 28 | SUCCESS | 149092.463011 | 8623.000000 | 0 | - |
| 29 | SUCCESS | 147607.667214 | 8510.000000 | 0 | - |
| 30 | SUCCESS | 147129.635075 | 8593.000000 | 0 | - |

### Strategy: ThreadLocal | Concurrency: 1000 | Requests per Client: 1000 | Total Requests: 1000000

| Run | Status | Throughput (ops/sec) | p99 (us) | Errors | Reason |
|---:|---|---:|---:|---:|---|
| 1 | SUCCESS | 37490.079422 | 251152.000000 | 0 | - |
| 2 | SUCCESS | 27400.101659 | 292386.000000 | 0 | - |
| 3 | SUCCESS | 28939.994596 | 273662.000000 | 0 | - |
| 4 | SUCCESS | 28329.049808 | 266043.000000 | 0 | - |
| 5 | SUCCESS | 25841.722007 | 276348.000000 | 0 | - |
| 6 | SUCCESS | 26184.948026 | 292246.000000 | 0 | - |
| 7 | SUCCESS | 28493.660992 | 272540.000000 | 0 | - |
| 8 | SUCCESS | 28631.846744 | 285620.000000 | 0 | - |
| 9 | SUCCESS | 25681.237768 | 274385.000000 | 0 | - |
| 10 | SUCCESS | 27009.539050 | 245698.000000 | 0 | - |
| 11 | SUCCESS | 28933.521850 | 244308.000000 | 0 | - |
| 12 | SUCCESS | 25559.849743 | 271492.000000 | 0 | - |
| 13 | SUCCESS | 26424.075131 | 244155.000000 | 0 | - |
| 14 | SUCCESS | 30623.433193 | 266817.000000 | 0 | - |
| 15 | SUCCESS | 28700.683191 | 271264.000000 | 0 | - |
| 16 | SUCCESS | 27583.174237 | 279541.000000 | 0 | - |
| 17 | SUCCESS | 30782.181465 | 275492.000000 | 0 | - |
| 18 | SUCCESS | 28844.770129 | 277968.000000 | 0 | - |
| 19 | SUCCESS | 28259.122775 | 274139.000000 | 0 | - |
| 20 | SUCCESS | 31434.091094 | 254682.000000 | 0 | - |
| 21 | SUCCESS | 28246.573572 | 266328.000000 | 0 | - |
| 22 | SUCCESS | 27679.885237 | 268868.000000 | 0 | - |
| 23 | SUCCESS | 29246.849143 | 272782.000000 | 0 | - |
| 24 | SUCCESS | 28540.448594 | 273597.000000 | 0 | - |
| 25 | SUCCESS | 28065.884247 | 282942.000000 | 0 | - |
| 26 | SUCCESS | 30043.137214 | 260393.000000 | 0 | - |
| 27 | SUCCESS | 28270.708069 | 262560.000000 | 0 | - |
| 28 | SUCCESS | 27154.090066 | 264426.000000 | 0 | - |
| 29 | SUCCESS | 30793.119637 | 270285.000000 | 0 | - |
| 30 | SUCCESS | 27847.860981 | 269911.000000 | 0 | - |

## Section 3 - Computed Metrics

- CI formula used: mean ± (1.96 x stddev / sqrt(50))

### Disabled - 100 Clients

Throughput:
- Mean: 36611.528644
- Stddev: 8584.735023
- CV: 0.234482
- CI95: [34231.961499, 38991.095790]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 28714.766667
- Stddev: 6024.696688
- CV: 0.209812
- CI95: [27044.805945, 30384.727389]
- Sample Size (successful runs): 30
- Failed Runs: 0

### Disabled - 500 Clients

Throughput:
- Mean: 147143.976597
- Stddev: 3787.562450
- CV: 0.025741
- CI95: [146094.117848, 148193.835345]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 8900.200000
- Stddev: 429.961538
- CV: 0.048309
- CI95: [8781.020742, 9019.379258]
- Sample Size (successful runs): 30
- Failed Runs: 0

### Disabled - 1000 Clients

Throughput:
- Mean: 47091.368160
- Stddev: 34809.807930
- CV: 0.739197
- CI95: [37442.581674, 56740.154646]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 226996.700000
- Stddev: 72927.860916
- CV: 0.321273
- CI95: [206782.128283, 247211.271717]
- Sample Size (successful runs): 30
- Failed Runs: 0

### GlobalMutex - 100 Clients

Throughput:
- Mean: 27764.705092
- Stddev: 25617.469791
- CV: 0.922663
- CI95: [20663.904742, 34865.505442]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 27128.833333
- Stddev: 5797.646080
- CV: 0.213708
- CI95: [25521.807829, 28735.858838]
- Sample Size (successful runs): 30
- Failed Runs: 0

### GlobalMutex - 500 Clients

Throughput:
- Mean: 31921.851325
- Stddev: 3687.522688
- CV: 0.115517
- CI95: [30899.722184, 32943.980466]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 132980.733333
- Stddev: 5263.863966
- CV: 0.039584
- CI95: [131521.664682, 134439.801984]
- Sample Size (successful runs): 30
- Failed Runs: 0

### GlobalMutex - 1000 Clients

Throughput:
- Mean: 32038.433293
- Stddev: 1965.755400
- CV: 0.061356
- CI95: [31493.553696, 32583.312891]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 260195.233333
- Stddev: 8392.037403
- CV: 0.032253
- CI95: [257869.079244, 262521.387423]
- Sample Size (successful runs): 30
- Failed Runs: 0

### Sharded - 100 Clients

Throughput:
- Mean: 43980.181617
- Stddev: 30681.576118
- CV: 0.697623
- CI95: [35475.682609, 52484.680625]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 22872.366667
- Stddev: 8646.880324
- CV: 0.378049
- CI95: [20475.573723, 25269.159610]
- Sample Size (successful runs): 30
- Failed Runs: 0

### Sharded - 500 Clients

Throughput:
- Mean: 35647.156977
- Stddev: 21928.226888
- CV: 0.615147
- CI95: [29568.962587, 41725.351366]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 127441.100000
- Stddev: 23083.395469
- CV: 0.181130
- CI95: [121042.709216, 133839.490784]
- Sample Size (successful runs): 30
- Failed Runs: 0

### Sharded - 1000 Clients

Throughput:
- Mean: 31895.775837
- Stddev: 1465.156159
- CV: 0.045936
- CI95: [31489.655270, 32301.896405]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 255582.500000
- Stddev: 10055.763881
- CV: 0.039344
- CI95: [252795.184459, 258369.815541]
- Sample Size (successful runs): 30
- Failed Runs: 0

### ThreadLocal - 100 Clients

Throughput:
- Mean: 36859.122201
- Stddev: 35977.091475
- CV: 0.976070
- CI95: [26886.781223, 46831.463178]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 25817.466667
- Stddev: 8860.570690
- CV: 0.343201
- CI95: [23361.441776, 28273.491558]
- Sample Size (successful runs): 30
- Failed Runs: 0

### ThreadLocal - 500 Clients

Throughput:
- Mean: 148949.743028
- Stddev: 1256.064014
- CV: 0.008433
- CI95: [148601.579847, 149297.906210]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 8586.666667
- Stddev: 131.549475
- CV: 0.015320
- CI95: [8550.203013, 8623.130321]
- Sample Size (successful runs): 30
- Failed Runs: 0

### ThreadLocal - 1000 Clients

Throughput:
- Mean: 28567.854655
- Stddev: 2264.156586
- CV: 0.079255
- CI95: [27940.262468, 29195.446841]
- Sample Size (successful runs): 30
p99 Latency:
- Mean: 269401.000000
- Stddev: 12359.994616
- CV: 0.045880
- CI95: [265974.984285, 272827.015715]
- Sample Size (successful runs): 30
- Failed Runs: 0

## Section 4 - Stability Classification

| Strategy | Clients | Metric | CV | Label |
|---|---:|---|---:|---|
| Disabled | 100 | Throughput | 0.234482 | Moderate |
| Disabled | 100 | p99 Latency | 0.209812 | Moderate |
| Disabled | 500 | Throughput | 0.025741 | Stable |
| Disabled | 500 | p99 Latency | 0.048309 | Stable |
| Disabled | 1000 | Throughput | 0.739197 | Unstable |
| Disabled | 1000 | p99 Latency | 0.321273 | Unstable |
| GlobalMutex | 100 | Throughput | 0.922663 | Unstable |
| GlobalMutex | 100 | p99 Latency | 0.213708 | Moderate |
| GlobalMutex | 500 | Throughput | 0.115517 | Moderate |
| GlobalMutex | 500 | p99 Latency | 0.039584 | Stable |
| GlobalMutex | 1000 | Throughput | 0.061356 | Stable |
| GlobalMutex | 1000 | p99 Latency | 0.032253 | Stable |
| Sharded | 100 | Throughput | 0.697623 | Unstable |
| Sharded | 100 | p99 Latency | 0.378049 | Unstable |
| Sharded | 500 | Throughput | 0.615147 | Unstable |
| Sharded | 500 | p99 Latency | 0.181130 | Moderate |
| Sharded | 1000 | Throughput | 0.045936 | Stable |
| Sharded | 1000 | p99 Latency | 0.039344 | Stable |
| ThreadLocal | 100 | Throughput | 0.976070 | Unstable |
| ThreadLocal | 100 | p99 Latency | 0.343201 | Unstable |
| ThreadLocal | 500 | Throughput | 0.008433 | Stable |
| ThreadLocal | 500 | p99 Latency | 0.015320 | Stable |
| ThreadLocal | 1000 | Throughput | 0.079255 | Stable |
| ThreadLocal | 1000 | p99 Latency | 0.045880 | Stable |

## Section 5 - Validation Checks

### 5.1 Outliers (value > 3x median)

- Disabled/c100 Throughput median=35024.267363
  - none
- Disabled/c100 p99 median=29403.000000
  - none
- Disabled/c500 Throughput median=147371.769366
  - none
- Disabled/c500 p99 median=8775.000000
  - none
- Disabled/c1000 Throughput median=35379.547989
  - run 1: throughput=148265.743003
  - run 2: throughput=148443.575604
  - run 3: throughput=148466.910278
- Disabled/c1000 p99 median=250578.000000
  - none
- GlobalMutex/c100 Throughput median=21046.830146
  - run 1: throughput=156282.813286
- GlobalMutex/c100 p99 median=27939.500000
  - none
- GlobalMutex/c500 Throughput median=31079.729629
  - none
- GlobalMutex/c500 p99 median=133884.500000
  - none
- GlobalMutex/c1000 Throughput median=31520.634835
  - none
- GlobalMutex/c1000 p99 median=260446.500000
  - none
- Sharded/c100 Throughput median=27170.714132
  - run 1: throughput=150321.980362
  - run 9: throughput=84222.861295
  - run 15: throughput=86054.721240
- Sharded/c100 p99 median=26375.500000
  - none
- Sharded/c500 Throughput median=31778.948377
  - run 1: throughput=151556.557823
- Sharded/c500 p99 median=131924.000000
  - none
- Sharded/c1000 Throughput median=31766.168960
  - none
- Sharded/c1000 p99 median=256154.000000
  - none
- ThreadLocal/c100 Throughput median=21905.940074
  - run 1: throughput=151887.832707
  - run 2: throughput=147949.138365
  - run 16: throughput=79999.157321
  - run 23: throughput=100945.688688
- ThreadLocal/c100 p99 median=28616.000000
  - none
- ThreadLocal/c500 Throughput median=148850.305083
  - none
- ThreadLocal/c500 p99 median=8599.500000
  - none
- ThreadLocal/c1000 Throughput median=28299.878938
  - none
- ThreadLocal/c1000 p99 median=271378.000000
  - none

### 5.2 Distribution Shape (skew check: mean/median >= 1.2)

- Disabled/c100 Throughput: mean=36611.528644, median=35024.267363, ratio=1.045319, skewed=NO
- Disabled/c100 p99: mean=28714.766667, median=29403.000000, ratio=0.976593, skewed=NO
- Disabled/c500 Throughput: mean=147143.976597, median=147371.769366, ratio=0.998454, skewed=NO
- Disabled/c500 p99: mean=8900.200000, median=8775.000000, ratio=1.014268, skewed=NO
- Disabled/c1000 Throughput: mean=47091.368160, median=35379.547989, ratio=1.331034, skewed=YES
- Disabled/c1000 p99: mean=226996.700000, median=250578.000000, ratio=0.905892, skewed=NO
- GlobalMutex/c100 Throughput: mean=27764.705092, median=21046.830146, ratio=1.319187, skewed=YES
- GlobalMutex/c100 p99: mean=27128.833333, median=27939.500000, ratio=0.970985, skewed=NO
- GlobalMutex/c500 Throughput: mean=31921.851325, median=31079.729629, ratio=1.027096, skewed=NO
- GlobalMutex/c500 p99: mean=132980.733333, median=133884.500000, ratio=0.993250, skewed=NO
- GlobalMutex/c1000 Throughput: mean=32038.433293, median=31520.634835, ratio=1.016427, skewed=NO
- GlobalMutex/c1000 p99: mean=260195.233333, median=260446.500000, ratio=0.999035, skewed=NO
- Sharded/c100 Throughput: mean=43980.181617, median=27170.714132, ratio=1.618661, skewed=YES
- Sharded/c100 p99: mean=22872.366667, median=26375.500000, ratio=0.867182, skewed=NO
- Sharded/c500 Throughput: mean=35647.156977, median=31778.948377, ratio=1.121722, skewed=NO
- Sharded/c500 p99: mean=127441.100000, median=131924.000000, ratio=0.966019, skewed=NO
- Sharded/c1000 Throughput: mean=31895.775837, median=31766.168960, ratio=1.004080, skewed=NO
- Sharded/c1000 p99: mean=255582.500000, median=256154.000000, ratio=0.997769, skewed=NO
- ThreadLocal/c100 Throughput: mean=36859.122201, median=21905.940074, ratio=1.682609, skewed=YES
- ThreadLocal/c100 p99: mean=25817.466667, median=28616.000000, ratio=0.902204, skewed=NO
- ThreadLocal/c500 Throughput: mean=148949.743028, median=148850.305083, ratio=1.000668, skewed=NO
- ThreadLocal/c500 p99: mean=8586.666667, median=8599.500000, ratio=0.998508, skewed=NO
- ThreadLocal/c1000 Throughput: mean=28567.854655, median=28299.878938, ratio=1.009469, skewed=NO
- ThreadLocal/c1000 p99: mean=269401.000000, median=271378.000000, ratio=0.992715, skewed=NO

### 5.3 Critical Anomalies

- ThreadLocal p99 < Disabled p99 at c100: CONFIRMED (ThreadLocal=25817.466667, Disabled=28714.766667)
- Sharded worst latency at c100: NOT CONFIRMED (worst=Disabled, value=28714.766667)
- ThreadLocal p99 < Disabled p99 at c500: CONFIRMED (ThreadLocal=8586.666667, Disabled=8900.200000)
- Sharded worst latency at c500: NOT CONFIRMED (worst=GlobalMutex, value=132980.733333)
- ThreadLocal p99 < Disabled p99 at c1000: NOT CONFIRMED (ThreadLocal=269401.000000, Disabled=226996.700000)
- Sharded worst latency at c1000: NOT CONFIRMED (worst=ThreadLocal, value=269401.000000)
- Overall ThreadLocal p99 < Disabled p99: NOT CONFIRMED
- Overall Sharded worst latency: NOT CONFIRMED

## Section 6 - Experiment Integrity Report

- Baseline reference directory: results/final_experiment/20260413_201218
- Baseline average CV: throughput=0.383538, p99=0.852380
- Current average CV: throughput=0.376785, p99=0.155655
- Baseline average relative CI width: throughput=0.212622, p99=0.472536
- Current average relative CI width: throughput=0.208879, p99=0.086291
- Did increasing requests stabilize results?: YES
- Did CV decrease?: YES
- Did CI tighten?: YES
- Are results now reliable?: NO

