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
effect. Randomized Multiple Interleaved Trials (RMIT) — a technique
defined by Abedi & Brecht, "Conducting Repeatable Experiments in Highly
Variable Cloud Computing Environments," ICPE 2017, not invented for this
project — fixes this: every repetition executes all (strategy,
concurrency) pairs in a fresh random order, so time-varying machine state
is spread evenly across strategies instead of confounded with them. See
`docs/paper_draft.md` §1 for the full related-work comparison, including
Laaber et al. (2019), who apply the same technique to cloud
microbenchmarking.

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

By default the script builds `server` and the `benchmarks/` client itself
before running (`cargo build --release`, once, at startup) — you do not
need to build either binary by hand first. Pass `--skip-build` to skip
this (the cloud-VM commands later in this doc do, since those binaries
were just built explicitly in the preceding step).

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

## Alternative: running on a cloud VM

Step 2's manual checklist exists to work around a constraint of running
*on the machine you're also using* — an agent session or your own
foreground work shares CPU/RAM with the benchmark no matter how many
other apps you close. A disposable cloud VM sidesteps this by construction:
nothing else runs on it, so there's no checklist to satisfy.

`experiment_results_rmit_azure/` was produced this way, on a dedicated
Azure `Standard_D4s_v6` VM (4 vCPU / 16GB RAM, Central India), torn down
immediately after the run. Rough steps, if repeating this:

```bash
# from wherever you're driving the VM from (needs az CLI + az login)
az group create --name <rg-name> --location centralindia
az vm create --resource-group <rg-name> --name rmit-bench-vm \
  --image Ubuntu2404 --size Standard_D4s_v6 \
  --admin-username <user> --ssh-key-values ~/.ssh/<key>.pub \
  --os-disk-size-gb 30 --storage-sku StandardSSD_LRS \
  --public-ip-sku Standard --nsg-rule SSH

# on the VM (via ssh)
sudo apt-get update -qq && sudo apt-get install -y gcc git curl build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source $HOME/.cargo/env
git clone --branch research/rmit-redesign --single-branch \
  https://github.com/Saksham932007/RustRedis.git ~/RustRedis
cd ~/RustRedis
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
RMIT_ENVIRONMENT_LABEL=azure-vm-Standard_D4s_v6-centralindia \
  python3 benchmarks/run_rmit_experiment.py --output-dir experiment_results_rmit_azure \
  --runs 30 --concurrency 100,200,300,400,500,600,700,1000 --skip-build

# pull results back, then IMMEDIATELY tear down:
az group delete --name <rg-name> --yes --no-wait
```

Set `RMIT_ENVIRONMENT_LABEL` before running so `metadata_rmit.json`'s
`machine_specs.environment` field records which environment produced the
data (the script itself detects real CPU/memory from `/proc`, but has no
way to know "this is a cloud VM" vs "this is the laptop" without a hint).

Notes:
- A brand-new Azure subscription may reject every VM size with
  `SkuNotAvailable: Capacity Restrictions` regardless of quota shown —
  this is a new-account deployment hold, not a real capacity or quota
  problem. Submitting a quota-increase request (Help + Support → Service
  and subscription limits (quotas) → Compute-VM) against whatever series
  the portal offers for your region, even for a small amount, is what
  clears it (usually within minutes).
- The repo is public, so the VM can `git clone` it directly with no
  credentials.
- **Always delete the resource group as soon as you've pulled the
  results.** A D4s_v6 in Central India costs about $0.21/hour — trivial
  for a ~2 hour run, but there's no reason to let it idle afterward.

## Raise the file-descriptor limit before testing high concurrency

`experiment_results_rmit_advanced/` pushed concurrency up to 3000 clients
on a `Standard_D8s_v6` (8 vCPU / 32GB RAM) and added `--workloads
mixed,read-heavy,write-heavy` as a third RMIT dimension:

```bash
RMIT_ENVIRONMENT_LABEL=azure-vm-Standard_D8s_v6-centralindia \
  python3 benchmarks/run_rmit_experiment.py --output-dir experiment_results_rmit_advanced \
  --runs 15 --concurrency 100,250,500,750,1000,1500,2000,3000 \
  --workloads mixed,read-heavy,write-heavy --skip-build
```

The first attempt at this silently failed: every run above roughly
c=1000 reported exactly `0 ops/sec`, with rc=0 and a validly-formed JSON
result — nothing about it looked like a crash. The cause was the default
open-file-descriptor soft limit on a fresh Ubuntu VM (`ulimit -n` = 1024)
being far below the concurrency levels being tested; every client
thread's `connect()` failed, and the benchmark client counts connection
failures into its error total rather than aborting the run, so it
"succeeded" while measuring nothing.

`run_rmit_experiment.py` now calls `raise_fd_limit()` once at startup,
which raises its own `RLIMIT_NOFILE` soft limit toward the hard limit —
subprocesses (server and benchmark client) inherit this automatically, no
root or `/etc/security/limits.conf` edit needed. It also prints a warning
if the achieved limit still can't cover the highest requested concurrency
level. **Before trusting a run at concurrency above ~1000, check
`ulimit -Hn` on the machine (or container) it'll run on**, and look at a
1-repetition dry run's per-run throughput numbers by eye — a suspiciously
*clean*, *uniform* zero across many runs is this failure mode, not noise.
