# RMIT Experiment Protocol (Linux i3-10110U, 8GB RAM)

This protocol replaces the fixed-order v12 matrix runner
(`benchmarks/run_final_experiment_v12.py`) with a randomized, interleaved
design, and targets the hardware this phase of the project actually runs
on: an Intel Core i3-10110U (2 cores / 4 threads) with 8GB DDR4 RAM and a
512GB SSD, not the 8-core Apple M2 the earlier `macos_m2_experiment_protocol.md`
was written for.

## Why the redesign

The v5/v12 runner completes all repetitions of one (strategy, concurrency)
configuration before moving to the next. If the machine drifts between a
"fast" and "slow" state over the course of a multi-hour run (thermal
throttling, background OS activity, frequency scaling), that drift gets
absorbed into whichever configurations happen to run during the slow
window — it looks like a strategy effect when it is actually a time
effect. Randomized Multiple Interleaved Trials (RMIT) fixes this: every
repetition executes all (strategy, concurrency) pairs in a fresh random
order, so time-varying machine state is spread evenly across strategies
instead of confounded with them.

## Step 1 — Randomized runner

Use `benchmarks/run_rmit_experiment.py`. It:

- Builds the full (strategy x concurrency) matrix (6 strategies x however
  many concurrency levels you pass).
- For each repetition, draws an independent random permutation of that
  matrix and runs it in that order.
- Restarts the server before every single run (unavoidable once order is
  shuffled — every run is potentially a strategy switch).
- Snapshots machine state (`benchmarks/system_state.py`) immediately
  before each run and writes it alongside that run's throughput/latency
  in `raw_data_rmit.csv`.

Defaults are scaled for this laptop: concurrency levels
`25,50,100,150,200,300,400,500` (the v12 matrix went up to 1000 clients on
an 8-core machine — on 4 threads, 1000 concurrent blocking-I/O client
threads is mostly measuring OS thread-scheduling overhead, not the
server), and 15 repetitions by default instead of 30. Both are
overridable:

```bash
python3 benchmarks/run_rmit_experiment.py \
  --runs 30 \
  --concurrency 25,50,100,150,200,300,400,500 \
  --output-dir experiment_results_rmit
```

The run is resumable: if it's interrupted, rerunning the same command
(same `--output-dir`) skips (block, strategy, concurrency) combinations
already recorded in `raw_data_rmit.csv`.

## Step 2 — Control the machine (manual checklist, do this before starting)

This is on you to do physically before kicking off a run — nothing here
can flip these settings from inside a script:

- [ ] Laptop plugged into AC power (not on battery — check `ac_online` in
      the logged state confirms this after the fact).
- [ ] Sleep/screen-lock/suspend disabled for the duration of the run.
- [ ] Every other application closed (browser, editors, chat apps).
- [ ] Wi-Fi/Bluetooth idle (turn off if not needed for the run itself).
- [ ] No OS update, backup, or indexing job scheduled to kick in mid-run
      (on Fedora: check `systemctl list-timers`, pause `tracker-miner-fs`
      / `mlocate` cron if present).
- [ ] No antivirus/malware scan scheduled.
- [ ] If you can, run `benchmarks/src/main.rs`'s client (`rustredis-bench`)
      from a second machine, or at minimum pin server and client to
      different cores with `taskset` so client load doesn't compete with
      server threads for the same 4 hardware threads:
      ```bash
      taskset -c 0,1 ./target/release/server &
      taskset -c 2,3 ./target/release/rustredis-bench ...
      ```

## Step 3 — Machine state is logged automatically

Every row in `raw_data_rmit.csv` includes CPU frequency (mean/max MHz),
CPU governor, max thermal-zone temperature, memory/swap availability,
1/5/15-minute load average, and AC/battery status
(`benchmarks/system_state.py`). This is what turns "we saw two throughput
states" into "we saw two states, and here's what was different about the
machine when the slow one happened."

## Step 4 — Testing what's causing the two states

`benchmarks/hardware_hypothesis_check.sh` runs one sustained load against
an already-running server and logs frequency/temperature/load/memory once
per second, with the three hypotheses annotated in its own output:

- **Hypothesis A — thermal throttling**: run once as-is, once with a fan
  or cooling pad (or in a cooler room), and diff the two CSVs. If
  `cpu_freq_mean_mhz` drops as `temp_max_c` climbs in the unfanned run but
  stays flat in the cooled run, that's throttling.
- **Hypothesis B — frequency/governor instability**: this CPU has no
  efficiency cores, so the macOS P/E-core scheduling hypothesis doesn't
  apply directly; the Linux analogue is turbo-boost/governor bouncing.
  Compare `cpupower frequency-set -g performance` (pinned governor)
  against the default `powersave`/`schedutil` governor, and compare
  `taskset`-pinned vs unpinned server placement.
- **Hypothesis C — memory pressure**: this machine only has 8GB. Watch
  `mem_available_kb` and `swap_free_kb` during the run; if available
  memory drops below roughly 500MB or swap starts draining, memory
  pressure is a plausible contributor, especially at higher concurrency
  levels where each client thread and each server-side connection buffer
  adds up.

You don't need to prove the exact cause for the paper — even a partial
answer (e.g., "throughput drops correlate with load_1m > 3.0 but not with
temperature") is stronger than leaving the two states unexplained.

## Step 5 — Full matrix rerun

Once the manual checklist is done and you've decided on repetitions:

```bash
python3 benchmarks/run_rmit_experiment.py --runs 30 --output-dir experiment_results_rmit
```

This is the dataset the paper reports. Expect this to take a while —
every run restarts the server, so budget roughly (startup + benchmark +
cooldown) x (strategies x concurrency levels x repetitions) seconds.

## Step 6 — Analysis

```bash
python3 benchmarks/analyze_rmit_results.py --input experiment_results_rmit/raw_data_rmit.csv
```

This reports medians with bootstrap 95% CIs (not just mean +/- stddev),
flags any (strategy, concurrency) configuration whose throughput
distribution looks bimodal and reports the two states separately instead
of averaging over them, and computes paired throughput ratios between
each strategy and a baseline (`disabled` by default) **within the same
repetition block**, which is the comparison RMIT is designed to make
valid — it controls for whatever time-varying condition caused block N to
be fast or slow, since every strategy in block N saw the same condition.

## Machine used

- CPU: Intel Core i3-10110U, 2 cores / 4 threads, up to 4.1GHz
- RAM: 8GB DDR4
- Storage: 512GB SSD
- OS: Linux (Fedora, kernel 6.19)

This replaces the Apple M2 numbers reported in `reports/final_experiment_v5.md`
and the v12 matrix in `experiment_results_v12/` as the machine target for
all runs produced from this protocol onward. Older results remain valid
for their own hardware but are not directly comparable to RMIT-protocol
runs on this laptop.
