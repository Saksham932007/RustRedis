# RMIT Experiment Protocol

Protocol for the Randomized Multiple Interleaved Trials (RMIT) experiments in
`experiments/`, run on dedicated Azure VMs (`Standard_D4s_v6`, `Standard_D8s_v6`).

## Why the redesign

The v5/v12 runner completes all repetitions of one (strategy, concurrency)
configuration before moving to the next. If the machine drifts between a
"fast" and "slow" state over the course of a multi-hour run (thermal
throttling, background OS activity, frequency scaling), that drift gets
absorbed into whichever configurations happen to run during the slow
window — it looks like a strategy effect when it is actually a time
effect. Randomized Multiple Interleaved Trials (RMIT) — a technique
proposed by Abedi, Heard & Brecht (2015) and shown necessary for cloud
environments by Abedi & Brecht, "Conducting Repeatable Experiments in
Highly Variable Cloud Computing Environments," ICPE 2017, not invented for
this project — fixes this: every repetition executes all (strategy,
concurrency) pairs in a fresh random order, so time-varying machine state
is spread evenly across strategies instead of confounded with them. See
`docs/paper_draft.md` §1 for the full related-work comparison, including
Laaber et al. (2019), who apply the same technique to cloud
microbenchmarking.

## Runner

Use `scripts/run_rmit_experiment.py`. It:

- Builds the full (strategy x concurrency x workload) matrix.
- For each repetition, draws an independent random permutation of that matrix
  and runs it in that order.
- Restarts the server before every single run (unavoidable once order is
  shuffled — every run is potentially a strategy switch).
- Snapshots machine state (`scripts/system_state.py`) immediately before each
  run and writes it alongside that run's throughput/latency in
  `raw_data_rmit.csv`.

The run is resumable: rerunning the same command (same `--output-dir`) skips
(block, strategy, concurrency, workload) combinations already recorded. By
default the script builds `server` and the `benchmarks/` client itself; pass
`--skip-build` if they were already built.

## Analysis

```bash
python3 scripts/analyze_rmit_results.py --input experiments/azure_d4s_v6/raw_data_rmit.csv
```

Reports medians with bootstrap 95% CIs, flags (strategy, concurrency)
configurations whose throughput distribution looks bimodal, and computes paired
throughput ratios between each strategy and a baseline (`disabled` by default)
**within the same repetition block** — the comparison RMIT is designed to make
valid, since every strategy in a block saw the same machine conditions.

Machine state (CPU frequency, governor, temperature, memory, load) is logged in
every row of `raw_data_rmit.csv`. `scripts/hardware_hypothesis_check.sh` logs it
once per second under sustained load, for diagnosing throttling, governor, or
memory-pressure effects.

## Provisioning a dedicated Azure VM

Running on the machine you also use for other work shares CPU/RAM with the benchmark no
matter how many other apps you close. A disposable cloud VM sidesteps this by
construction: nothing else runs on it.

`experiments/azure_d4s_v6/` was produced this way, on a dedicated
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
git clone --single-branch \
  https://github.com/Saksham932007/RustRedis.git ~/RustRedis
cd ~/RustRedis
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
RMIT_ENVIRONMENT_LABEL=azure-vm-Standard_D4s_v6-centralindia \
  python3 scripts/run_rmit_experiment.py --output-dir experiments/azure_d4s_v6 \
  --runs 30 --concurrency 100,200,300,400,500,600,700,1000 --skip-build

# pull results back, then IMMEDIATELY tear down:
az group delete --name <rg-name> --yes --no-wait
```

Set `RMIT_ENVIRONMENT_LABEL` before running so `metadata_rmit.json`'s
`machine_specs.environment` field records which environment produced the
data (the script itself detects real CPU/memory from `/proc`, but has no
way to know which VM it is running on without a hint).

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

`experiments/azure_d8s_v6/run_00/` pushed concurrency up to 3000 clients
on a `Standard_D8s_v6` (8 vCPU / 32GB RAM) and added `--workloads
mixed,read-heavy,write-heavy` as a third RMIT dimension:

```bash
RMIT_ENVIRONMENT_LABEL=azure-vm-Standard_D8s_v6-centralindia \
  python3 scripts/run_rmit_experiment.py --output-dir experiments/azure_d8s_v6/run_00 \
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
