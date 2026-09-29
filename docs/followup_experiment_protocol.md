# Follow-up experiment: splitting the confounded design axes

This protocol runs the experiment that resolves the principal internal-validity
threat in [paper_design_axes.md](paper_design_axes.md) §3.3 and §7.

## What question this answers

The six-strategy design prices four axes, but two of the four contrasts bundle
an extra cost with the axis they are meant to isolate:

| Paper axis | Contrast used | What it actually bundles |
|---|---|---|
| CARD | `sharded_n` − `sharded_2key` | map cardinality **+** per-operation owned-key allocation |
| PAYLOAD | `hdr_histogram` − `thread_local` | HDR histogram **+** per-operation owned-key allocation |

So the paper can say "high-cardinality dynamic labeling costs 1.66× what the
synchronization axis saves" but cannot say how much of that is cardinality and
how much is the allocation that dynamic labeling forces. This batch separates
them.

## The two new collectors

Both are in `src/command_metrics.rs`.

**`sharded_bucketed`** — a `DashMap<String, CommandStat>` keyed by
`"<CMD>#<fnv1a(logical key) mod C>"`, where `C` comes from
`RUSTREDIS_METRICS_CARDINALITY`. The point of the design is that
**per-operation work is identical at every `C`**: one hash of the logical key,
one modulo, one `format!`, one allocation, one map lookup — regardless of
whether `C` is 1 or 10,000. Only the number of live map entries changes. This is
a stronger instrument than sweeping the workload key space, which would vary the
request distribution alongside the cardinality.

**`thread_local_owned`** — a line-for-line twin of the HdrHistogram collector
(same owned `String` key, same TLS map, same 1000-record flush trigger, same
CAS-guarded flush on the same 100 ms cadence) with `CommandStat` substituted for
the histogram.

## The contrasts this batch enables

| New contrast | Isolates |
|---|---|
| `sharded_bucketed`@C=1 − `sharded_2key` | owned-key allocation alone (both hold one entry per command) |
| `sharded_bucketed`@C=10⁴ − `sharded_bucketed`@C=1 | map entry count alone (every other step identical) |
| `hdr_histogram` − `thread_local_owned` | the HDR histogram alone |
| `thread_local_owned` − `thread_local` | owned-key allocation alone, in the TLS path |

Reported together, `ALLOC + pure CARD` should reconstruct the paper's CARD, and
`ALLOC(tls) + pure PAYLOAD` should reconstruct its PAYLOAD — an internal
consistency check worth stating in the write-up either way.

`global_mutex` and `sharded_n` are kept in the recommended run purely as
**anchors**: if this batch does not reproduce the paper's CARD ≈ 0.87 pp and
SYNC ≈ 0.52 pp, something about the new setup differs and the new numbers should
not be trusted.

## Sizing

The paper's §5.2 result — that no axis responds to client concurrency over a 30×
range — means this batch does **not** need the full 8-concurrency × 3-workload
matrix. Three concurrency levels and one workload suffice, which keeps a run to
a few hours:

| | Main paper batch | This batch |
|---|---|---|
| Strategy configs | 6 | 12 (`all`, with 5 cardinalities) |
| Concurrency | 8 levels (100–3000) | 3 levels (100, 1000, 3000) |
| Workloads | 3 | 1 (mixed) |
| Repetitions | 15 | 15 |
| **Runs per VM** | 2,160 | **540** |

**Run it on 6 independently provisioned VMs.** One run resolves paired effects
of roughly 2%; the allocation axis may well be smaller than that, and
cluster-aware inference needs independent instances, not more blocks within one
(paper §4.3). Six runs is the point where the cluster-aware floor lands near
1%; go to 8–11 if the allocation axis comes back indistinguishable from zero.

## Provisioning (Azure)

Same family, size, and image as the main batch, so results stay comparable.
Provision solely for the run and delete immediately after. Vary the region
across VMs as the main batch did (Central India / South India / West US 3).

```bash
# from wherever you drive the VM (needs az CLI + az login)
RG=rmit-followup-01
REGION=centralindia          # vary per VM: centralindia | southindia | westus3

az group create --name $RG --location $REGION
az vm create --resource-group $RG --name rmit-followup-vm \
  --image Ubuntu2404 --size Standard_D8s_v6 \
  --admin-username <user> --ssh-key-values ~/.ssh/<key>.pub \
  --os-disk-size-gb 30 --storage-sku StandardSSD_LRS \
  --public-ip-sku Standard --nsg-rule SSH
```

On the VM:

```bash
sudo apt-get update -qq && sudo apt-get install -y gcc git curl build-essential python3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source $HOME/.cargo/env
git clone --single-branch https://github.com/Saksham932007/RustRedis.git ~/RustRedis
cd ~/RustRedis
cargo build --release --bin server
cargo build --release --manifest-path benchmarks/Cargo.toml
```

### Check the fd limit before anything else

This is the failure that silently produced `0 ops/sec` for the whole
high-concurrency half of the first main-batch attempt (paper §4.4). The runner
now raises `RLIMIT_NOFILE` itself and warns if it cannot, but confirm the hard
limit can cover the concurrency anyway:

```bash
ulimit -Hn        # must comfortably exceed 3000; raise it if not
```

### Smoke test before committing hours to the full run

One block, ~10 minutes. **Read the per-run throughput numbers with your own
eyes** before starting the real run — this is what catches the "clean but wrong"
class of failure that no amount of randomization detects:

```bash
python3 scripts/run_rmit_experiment.py \
  --strategies all --cardinalities 1,10,100,1000,10000 \
  --runs 1 --concurrency 100,3000 --workloads mixed \
  --key-space 10000 --skip-build --seed 1 \
  --output-dir /tmp/smoke

# every row should be a plausible ops/sec, never 0, and the achieved entry
# counts should track the requested cardinalities (2C + 2, saturating at the
# number of distinct keys actually touched)
column -s, -t /tmp/smoke/raw_data_rmit.csv | cut -c1-150
```

### The run

One per VM, with a distinct `--seed` and a label identifying the VM and region:

```bash
IDX=01                        # 01..06, distinct per VM
REGION=centralindia

RMIT_ENVIRONMENT_LABEL=azure-Standard_D8s_v6-$REGION-followup-$IDX \
python3 scripts/run_rmit_experiment.py \
  --strategies all \
  --cardinalities 1,10,100,1000,10000 \
  --runs 15 \
  --concurrency 100,1000,3000 \
  --workloads mixed \
  --key-space 10000 \
  --seed $((2000 + 10#$IDX)) \
  --skip-build \
  --output-dir experiments/azure_d8s_v6_followup/run_$IDX
```

Pull the results back, then tear the VM down immediately:

```bash
scp -r <user>@<ip>:~/RustRedis/experiments/azure_d8s_v6_followup/run_$IDX \
  experiments/azure_d8s_v6_followup/
az group delete --name $RG --yes --no-wait
```

## Analysis

`scripts/analyze_design_axes.py` already contains the follow-up section; it
activates automatically once `sharded_bucketed_c1` and `thread_local_owned` are
present in the data and stays silent otherwise, so it is safe to run against
either dataset.

```bash
python3 scripts/combine_iterations.py \
  --input-dir experiments/azure_d8s_v6_followup \
  --output-dir experiments/azure_d8s_v6_followup/pooled
python3 scripts/analyze_design_axes.py
```

It prints the four new contrasts with cluster-aware intervals and the
cardinality curve (`sharded_bucketed` against itself at rising `C`, which is the
headline of this batch).

## Two things the analysis must account for

**Achieved cardinality is capped by the key space.** The bucket is
`hash(logical key) mod C`, so no more than `--key-space` distinct buckets can
ever exist, and in practice only as many as the run actually touches. At
`C = 10,000` over a 10,000-key space a run reaches roughly 6,000–10,000. The
runner records the real figure per run in `achieved_metric_entries`; **key the
cardinality curve off that column, not off the requested `C`.**

**The achieved count includes exactly two control-plane entries** — the
harness's own `PING` health check and the `CMDSTAT` readback. Neither carries a
key hint, so each occupies one bucket at any `C`. Data-plane entries are
therefore `achieved_metric_entries − 2` (verified: C=10 → 22, C=100 → 202,
C=1000 → 1964). Their throughput contribution is a handful of operations against
millions, identical across strategies.

## What a clean result looks like

- **Anchors reproduce.** CARD ≈ 0.87 pp and SYNC ≈ 0.52 pp, within overlapping
  intervals of the main batch.
- **The reconstruction holds.** `ALLOC + pure CARD ≈ CARD`, and
  `ALLOC(tls) + pure PAYLOAD ≈ PAYLOAD`, within intervals.
- **The cardinality curve is monotone** in achieved entries, and its span from
  C=1 to C=10⁴ is the number that upgrades the paper's headline from "dynamic
  labeling costs 1.66× the synchronization axis" to "map cardinality *alone*
  costs X".

If the curve is flat and the allocation contrast carries the whole CARD axis,
that is an equally publishable result and a more surprising one — it would mean
the cost of a high-cardinality label is the per-operation key materialization,
not the map size, which points at a different fix (intern the label) than the
one the paper currently implies.
