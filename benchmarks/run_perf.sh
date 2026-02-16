#!/bin/bash
set -e

# Find PIDs
RUST_PID=$(pgrep -f "target/release/server" | head -n 1)
VALKEY_PID=$(pgrep valkey-server | head -n 1)

if [ -z "$RUST_PID" ]; then 
    echo "RustRedis server not found. Please start it with 'cargo run --release --bin server'"
    exit 1
fi

if [ -z "$VALKEY_PID" ]; then
    echo "Valkey server not found. Please start it on port 6380."
    exit 1
fi

echo "RustRedis PID: $RUST_PID"
echo "Valkey PID: $VALKEY_PID"

mkdir -p benchmarks/results

run_measurement() {
    TARGET_PID=$1
    NAME=$2
    CONC=$3
    PORT=$4
    
    echo "------------------------------------------------------------------"
    echo "Measuring $NAME components at c=$CONC (PID $TARGET_PID)"
    
    # Calculate requests per client to run for ~10-15 seconds
    # Assuming ~50k ops/sec mixed load -> 750k ops total
    TOTAL_OPS=1000000
    REQ_PER_CLIENT=$((TOTAL_OPS / CONC))
    
    # Start perf stat in background
    # We use sleep 10 to measure a 10s window of steady state
    echo "Starting perf stat..."
    perf stat -p $TARGET_PID \
        -e task-clock,context-switches,cpu-migrations,page-faults,cycles,instructions,branches,branch-misses,cache-references,cache-misses \
        -o "benchmarks/results/perf_${NAME}_c${CONC}.txt" \
        sleep 10 &
    PERF_PID=$!
    
    # Start load generator
    # We run slightly more ops to ensure load covers the 10s window
    echo "Starting load generator..."
    ./target/release/rustredis-bench \
        --host 127.0.0.1 \
        --port $PORT \
        --concurrency $CONC \
        --requests $REQ_PER_CLIENT \
        --workload mixed \
        > /dev/null 2>&1
        
    wait $PERF_PID
    echo "Measurement complete."
    cat "benchmarks/results/perf_${NAME}_c${CONC}.txt"
}

# RustRedis Measurements
run_measurement $RUST_PID "RustRedis" 100 6379
run_measurement $RUST_PID "RustRedis" 1000 6379

# Valkey Measurements
run_measurement $VALKEY_PID "Valkey" 100 6380
run_measurement $VALKEY_PID "Valkey" 1000 6380

echo "------------------------------------------------------------------"
echo "All hardware measurements specific to PID completed."
