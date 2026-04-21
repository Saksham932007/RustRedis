# V14 Data Package - Complete Missing Data for Paper Rewrite

## 1. Statistical Computations

### Computation 1A: Welch's t-test - Disabled vs Sharded-2key Throughput at 600c

Given:
- Disabled: mean = 217,593, CI = [216,905, 218,282], n = 30
- Sharded-2key: mean = 215,867, CI = [213,937, 217,797], n = 30
- SD back-calculation: SD = CI_halfwidth / 0.35777

Step 1: Back-calculate SDs

- Disabled CI halfwidth:

  (218,282 - 216,905) / 2 = 688.5

  SD_D = 688.5 / 0.35777 = 1924.4207

- Sharded-2key CI halfwidth:

  (217,797 - 213,937) / 2 = 1930

  SD_S = 1930 / 0.35777 = 5394.5272

Step 2: Welch t-statistic

t = (mean_D - mean_S) / sqrt((SD_D^2/n) + (SD_S^2/n))

Intermediate values:

- Mean difference:

  Delta = 217,593 - 215,867 = 1,726

- SD_D^2 / n = 123,446.5032
- SD_S^2 / n = 970,030.7941
- SE = sqrt(1,093,477.2973) = 1045.6946

So:

t = 1726 / 1045.6946 = 1.6506

Step 3: Welch-Satterthwaite df

df = ((SD_D^2/n + SD_S^2/n)^2) / (((SD_D^2/n)^2/(n-1)) + ((SD_S^2/n)^2/(n-1)))

= 36.2635

Step 4: Two-tailed p-value and Bonferroni result

- p = 0.10746 (two-tailed)
- Bonferroni threshold alpha_adj = 0.006
- Result: not significant (fails Bonferroni)

Step 5: Cohen's d

Pooled SD:

SD_pooled = sqrt(((n-1)SD_D^2 + (n-1)SD_S^2) / (2n-2)) = 4049.9580

d = 1726 / 4049.9580 = 0.4262

Step 6: Plain conclusion

This comparison does not survive Bonferroni correction.

---

### Computation 1B: Welch's t-test - Disabled vs Sharded-2key Throughput at 700c

Given:
- Disabled: mean = 217,472, CI = [216,644, 218,301], n = 30
- Sharded-2key: mean = 211,265, CI = [208,284, 214,246], n = 30

Step 1: SD back-calculation

- Disabled halfwidth:

  (218,301 - 216,644) / 2 = 828.5

  SD_D = 828.5 / 0.35777 = 2315.7336

- Sharded-2key halfwidth:

  (214,246 - 208,284) / 2 = 2981

  SD_S = 2981 / 0.35777 = 8332.1687

Step 2: Welch t-statistic

- Mean difference:

  Delta = 217,472 - 211,265 = 6,207

- SD_D^2 / n = 178,754.0659
- SD_S^2 / n = 2,314,167.8480
- SE = sqrt(2,492,921.9139) = 1578.8990

t = 6207 / 1578.8990 = 3.9312

Step 3: Welch df

df = 33.4535

Step 4: p-value and Bonferroni

- p = 0.000402 (two-tailed)
- alpha_adj = 0.006
- Significant after Bonferroni

Step 5: Cohen's d

SD_pooled = 6115.0494

d = 6207 / 6115.0494 = 1.0150

Step 6: Plain conclusion and implication

This comparison does survive Bonferroni correction.
Implication: the claim of near-zero overhead across all stable concurrency levels is not supported at 700 clients; a statistically significant throughput deficit appears at the upper boundary.

---

### Computation 1C: Welch's t-test - Disabled vs Sharded-2key p99 at 600c

Given:
- Disabled: mean = 6,600, CI = [6,306, 6,894], n = 30
- Sharded-2key: mean = 6,504, CI = [6,254, 6,754], n = 30

Step 1: SD back-calculation

- Disabled halfwidth:

  (6894 - 6306) / 2 = 294

  SD_D = 294 / 0.35777 = 821.7570

- Sharded-2key halfwidth:

  (6754 - 6254) / 2 = 250

  SD_S = 250 / 0.35777 = 698.7730

Step 2: Welch t-statistic

- Mean difference:

  Delta = 6600 - 6504 = 96

- SD_D^2 / n = 22509.4853
- SD_S^2 / n = 16276.1214
- SE = sqrt(38785.6067) = 196.9406

t = 96 / 196.9406 = 0.4875

Step 3: df

df = 56.5397

Step 4: p and Bonferroni

- p = 0.62782 (two-tailed)
- Not significant at alpha_adj = 0.006

Step 5: Cohen's d

SD_pooled = 762.7477

d = 96 / 762.7477 = 0.1259

Step 6: Plain conclusion

Does not survive Bonferroni correction.

---

### Computation 1D: Welch's t-test - Disabled vs Sharded-2key p99 at 700c

Given:
- Disabled: mean = 7,434, CI = [6,939, 7,930], n = 30
- Sharded-2key: mean = 8,420, CI = [8,013, 8,826], n = 30

Step 1: SD back-calculation

- Disabled halfwidth:

  (7930 - 6939) / 2 = 495.5

  SD_D = 495.5 / 0.35777 = 1384.9680

- Sharded-2key halfwidth:

  (8826 - 8013) / 2 = 406.5

  SD_S = 406.5 / 0.35777 = 1136.2048

Step 2: Welch t-statistic

- Mean difference:

  Delta = 7434 - 8420 = -986

- SD_D^2 / n = 63937.8783
- SD_S^2 / n = 43032.0468
- SE = sqrt(106969.9251) = 327.0626

t = -986 / 327.0626 = -3.0147

Step 3: df

df = 55.8662

Step 4: p and Bonferroni

- p = 0.003864 (two-tailed)
- Significant after Bonferroni (p < 0.006)

Step 5: Cohen's d

SD_pooled = 1266.7079

d = -986 / 1266.7079 = -0.7784

Step 6: Plain conclusion with direction

Significant after Bonferroni. Direction is unfavorable for Sharded-2key at 700c: p99 is higher by 986 us (about 13.26%).

---

### Computation 1E: Cohen's d - Disabled vs Sharded-2key Throughput at 500c

Given:
- Disabled mean = 217,873, CI = [217,245, 218,502], n = 30
- Sharded-2key mean = 217,089, CI = [216,472, 217,706], n = 30
- Existing test result: p = 0.086

Step 1: SDs

- Disabled halfwidth:

  (218502 - 217245) / 2 = 628.5

  SD_D = 628.5 / 0.35777 = 1756.7152

- Sharded-2key halfwidth:

  (217706 - 216472) / 2 = 617

  SD_S = 617 / 0.35777 = 1724.5717

Step 2: Pooled SD and d

SD_pooled = sqrt(((29)SD_D^2 + (29)SD_S^2) / 58) = 1740.7176

Mean difference:

Delta = 217873 - 217089 = 784 ops/s

d = 784 / 1740.7176 = 0.4504

Effect-size class (given thresholds): medium (0.3-0.5).

Practical interpretation:
- Absolute difference is 784 ops/s on a about 218k ops/s baseline (about 0.36%).
- Even if statistically significant, this absolute effect is likely operationally small in production throughput terms.

---

### Computation 1F: Cohen's d - Disabled vs Sharded-2key p99 at 500c

Given:
- Disabled mean = 5,494, CI = [5,330, 5,657], n = 30
- Sharded-2key mean = 5,343, CI = [5,225, 5,462], n = 30
- Existing test result: p = 0.150

Step 1: SDs

- Disabled halfwidth:

  (5657 - 5330) / 2 = 163.5

  SD_D = 163.5 / 0.35777 = 456.9975

- Sharded-2key halfwidth:

  (5462 - 5225) / 2 = 118.5

  SD_S = 118.5 / 0.35777 = 331.2184

Step 2: Pooled SD and d

SD_pooled = sqrt(((29)SD_D^2 + (29)SD_S^2) / 58) = 399.0942

Mean difference:

Delta = 5494 - 5343 = 151 us

d = 151 / 399.0942 = 0.3784

Effect-size class: medium (0.3-0.5).

Practical interpretation:
- Absolute difference is 151 us (about 2.75% of Disabled p99).
- This is modest in absolute latency terms, and the existing inferential test is not significant after correction.

---

### Computation 1G: Mann-Whitney U - Complete Reporting Fix

Given:
- Reported: U = 886, n1 = n2 = 30
- Maximum Umax = n1*n2 = 900

1) One-tailed interpretation
If the one-tailed alternative is HdrHistogram throughput < Disabled throughput, then:

U / 900 = 886 / 900 = 0.98444

Interpretation: 886 of 900 pairwise comparisons (98.44%) are in the hypothesized direction.

2) Rank-biserial correlation
Using requested formula:

r = 1 - (2U / (n1*n2)) = 1 - (2*886 / 900) = 1 - 1.96889 = -0.96889

Magnitude indicates an extremely large effect; sign depends on group ordering convention.

3) APA-compliant sentence
Mann-Whitney U = 886 (one-tailed, testing that HdrHistogram throughput < Disabled throughput), N1 = N2 = 30, p < 0.001, r = -0.969.

---

### Computation 1H: Mann-Whitney for HdrHistogram p99 at 500c

1) Why exact U cannot be computed from summary statistics
Exact Mann-Whitney U requires the raw run-level values (or full rank ordering) for all 60 observations. Means, CIs, and CVs are insufficient to reconstruct exact ranks.

2) Lower-bound argument from available data
HdrHistogram p99 mean = 55,977 us vs Disabled p99 mean = 5,494 us (about 10.19x).
CIs do not overlap: HdrHistogram [47,092, 64,861] vs Disabled [5,330, 5,657].
Given this extreme separation, U is almost certainly near the extreme (close to 900 under the directional hypothesis), and p < 0.001 is essentially certain even without exact raw ranks.

3) Placeholder sentence for paper
Mann-Whitney U [computed from raw data, value TBD by author], p < 0.001 (one-tailed), confirming HdrHistogram p99 significantly exceeds Disabled p99 at 500 clients.

4) Required flag
[AUTHOR ACTION REQUIRED - run Mann-Whitney on raw 30-run p99 values for HdrHistogram and Disabled at 500c. Replace placeholder with exact U and p-value.]

---

### Computation 1I: Shapiro-Wilk Assessment

1) Methodological statement for revised Section 4.6
Prior to applying Welch's t-test, normality of each distribution was assessed using the Shapiro-Wilk test (alpha = 0.05). Distributions with SW p < 0.05 were treated as non-normal and tested using Mann-Whitney U instead of Welch's t-test. CV values are reported as a supplementary indicator of distributional stability but are not used as a normality criterion.

2) SW results table shell

| Strategy | 400c | 500c | 600c | 700c |
|---|---|---|---|---|
| Disabled | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |
| Sharded-2key | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |
| HdrHistogram | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |
| GlobalMutex | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |
| ThreadLocal | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |
| Sharded-N | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ | W = ___, p = ___ |

3) Required flag
[AUTHOR ACTION REQUIRED - run shapiro.test() in R or scipy.stats.shapiro() in Python on each of the 30-run distributions for strategies at 400, 500, 600, 700c. Populate the SW table. Update test selections in Tables 5.2.1 and 5.2.2 accordingly.]

---

### Computation 1J: Consolidated Effect Size Table

| Comparison | Concurrency | Metric | Mean Difference | Cohen's d | Interpretation |
|---|---|---|---:|---:|---|
| Disabled vs Sharded-2key | 500c | Throughput | 784 ops/s | 0.450 | Medium standardized effect; absolute change is small (about 0.36%) |
| Disabled vs Sharded-2key | 500c | p99 | 151 us | 0.378 | Medium standardized effect; absolute p99 change is modest |
| Disabled vs Sharded-2key | 600c | Throughput | 1,726 ops/s | 0.426 | Medium standardized effect; inferential test not significant after Bonferroni |
| Disabled vs Sharded-2key | 600c | p99 | 96 us | 0.126 | Small effect |
| Disabled vs Sharded-2key | 700c | Throughput | 6,207 ops/s | 1.015 | Large effect; significant deficit for Sharded-2key |
| Disabled vs Sharded-2key | 700c | p99 | -986 us (Disabled - Sharded) | -0.778 | Large effect; Sharded-2key has higher p99 |

## 2. Text Blocks (2A-2V)

### 2A: Revised Title (3 candidates + recommendation)

Candidate 1:
**Bounded Telemetry Overhead in Concurrent Systems: Allocation-Free Paths and Concurrency-Induced Phase Transitions**

Candidate 2:
**When Observability Is Free and When It Fails: Contention Regimes in a Concurrent Key-Value Server**

Candidate 3:
**Observability Under Load: Near-Zero-Overhead Instrumentation and Collapse Thresholds in Concurrent Servers**

Recommended: Candidate 1.
Justification: It captures both core findings explicitly (near-zero overhead and phase-transition failures), uses implementation-neutral language suitable for a systems/performance venue, and remains under 20 words in the main title.

---

### 2B: Corrected Abstract (Full Replacement)

Observability is often treated as an inherent bottleneck in concurrent services, yet overhead depends strongly on data structure, aggregation strategy, and operating regime. We present a controlled benchmark study of six metrics-collection strategies in a Rust/Tokio key-value server across 100-1000 concurrent clients (30 runs per condition). Strategies include disabled instrumentation, sharded command-key aggregation (allocation-free key path), global mutex aggregation, thread-local buffering with periodic flush, high-cardinality sharded keys, and HdrHistogram-based tracking.

Primary confirmatory comparisons at 500 clients use Welch's t-tests for five strategies with Bonferroni correction. Disabled vs Sharded-2key is not significant for throughput (p = 0.086) or p99 latency (p = 0.15). Extending these comparisons with the same methodology shows no significant differences at 600 clients (throughput p = 0.107, p99 p = 0.628), but significant divergence at 700 clients (throughput p = 0.0004, p99 p = 0.0039). Thus, statistical equivalence with baseline holds through 400-600 clients, not across the full 400-700 range.

HdrHistogram exhibits a sharp degradation near 500 clients; because dispersion violates the parametric-stability threshold (throughput CV = 0.577), this comparison is reported with Mann-Whitney testing. Overall, results show that observability overhead is regime-dependent: near-zero overhead is achievable in bounded regions with allocation-free sharded aggregation, while abrupt phase transitions and heavy-tail behavior emerge in alternative designs under higher concurrency.

---

### 2C: macOS Version Footnote

1 macOS 26.4 denotes the exact operating-system version reported by the host at experiment time; Apple marketing names may differ across release channels. For reproducibility, the archived metadata includes OS build and kernel identifiers in addition to this numeric version.

---

### 2D: Revised Section 2.3 (Background - Sharded Concurrent Data Structures)

Sharding reduces lock contention by partitioning shared state into multiple independently synchronized regions, allowing concurrent updates to proceed in parallel when operations map to different shards [9, 17]. The effectiveness of sharding depends on the mapping between workload keys and shard indices: a balanced key distribution increases parallelism, while skewed or low-cardinality key spaces can concentrate traffic and recreate serialization.

DashMap is a concurrent hash map that applies shard-level synchronization internally, using per-shard read-write locking around hash-bucket state. This design exposes a practical middle ground between a single global lock and fully lock-free structures: updates contend only when they collide on the same shard, and independent shards can progress concurrently. Because shard assignment is hash-derived, both key type and key cardinality materially affect contention behavior.

From a theory perspective, sharding reduces contention when three properties hold simultaneously: sufficient key entropy relative to shard count, low per-operation overhead on the hot path, and bounded synchronization costs within each shard. If any of these fail, expected gains from partitioned locking may be diminished or negated.

---

### 2E: Revised Section 2.4 (Background - Deferred and Batched Aggregation)

Deferred and batched aggregation strategies move work away from the immediate request path by buffering updates and aggregating periodically. In principle, this can reduce synchronization frequency and improve throughput by amortizing update costs across batches. However, batching introduces additional queueing, flush, and coordination dynamics that may shift overhead rather than eliminate it.

Two distinct degradation patterns are theoretically possible. First, a strategy may exhibit an abrupt phase transition, transitioning from low-overhead behavior to high-contention/high-latency behavior once concurrency exceeds a threshold. Second, a strategy may remain consistently below baseline across all tested loads, indicating a structural overhead floor rather than threshold-triggered collapse.

These failure modes are analytically different and should be interpreted separately: phase-transition systems are regime-sensitive, whereas uniformly underperforming systems indicate persistent per-operation overhead that batching does not amortize sufficiently.

---

### 2F: Revised Section 3.3.3 Explanation of v10->v12 Reversal

The reversal between the earlier low-concurrency collapse result and the present near-baseline behavior is attributed to workload parameterization, not to a structural redesign of the sharded collector. The Sharded-2key implementation is structurally unchanged from commit git:624cb56 (Appendix A). The key change is operating regime: the present workload executes substantially more operations per client run, increasing total request-path work. Under this regime, fixed per-call overheads of the DashMap update path contribute a smaller fraction of end-to-end request cost, whereas at lower ops/client they can dominate measured throughput. In other words, the observed reversal is consistent with a shift from overhead-dominated measurement conditions to throughput-dominated conditions under otherwise similar collector structure. The implementation excerpt is moved to Appendix A for reference and removed from the design narrative.

[AUTHOR ACTION REQUIRED: Confirm that the explanation in Section 3.3.3 - that the v10 collapse was due to low ops/client causing per-call overhead to dominate, and the code structure is unchanged from git:624cb56 - is accurate and complete. If there was an additional code change not reflected here, document it precisely: (a) what file/function changed, (b) what the before and after code looks like, (c) why the change eliminates the contention. Without this confirmation, Section 3.3.3 as written may be incorrect.]

---

### 2G: Revised Section 6.1 Discussion - Sharded-2key (Full Replacement)

The Sharded-2key strategy demonstrates that near-baseline observability overhead is achievable within a bounded concurrency regime when three concrete properties are present: an allocation-free key type on the hot path (&'static str), DashMap's default shard partitioning (64 shards), and a low-cardinality two-command key space that distributes updates across shards. Under these conditions, Disabled and Sharded-2key are statistically indistinguishable at 500 and 600 clients for both throughput and p99 latency.

At 700 clients, however, the equivalence no longer holds: throughput differs significantly and p99 latency is significantly higher for Sharded-2key. The empirical result therefore supports a bounded claim: near-zero overhead at 400-600 clients, with measurable divergence emerging at the upper boundary. We hypothesize that this boundary effect reflects increasing shard-level contention and scheduling interaction under higher concurrent load, but this mechanism remains a hypothesis pending profiler-validated lock-contention traces.

The practical implication is not that sharded metrics are universally free, but that they can be effectively free within specific operating regimes when allocation and contention pathways are tightly constrained.

---

### 2H: Revised Section 6.2 Discussion - HdrHistogram Phase Transition

HdrHistogram exhibits phase-transition behavior in this server workload: performance transitions from a comparatively stable region to a degraded high-latency region as concurrency increases. The transition appears abrupt rather than gradual, indicating a thresholded contention process rather than smooth scaling loss.

We hypothesize that this transition is driven by atomic/CAS contention and coordination costs in shared histogram update paths under high concurrent write pressure. This mechanism is plausible given the observed dispersion and tail behavior, but it remains inferential without direct low-level contention profiling.

Systems with more cores may experience the transition at different concurrency levels; whether the relationship is linear, sub-linear, or dependent on scheduler topology requires empirical investigation.

Practical guidance: avoid placing shared histogram recording directly in the per-request hot path at high write concurrency. Prefer per-thread/local recording with periodic aggregation, and validate transition thresholds under production-like concurrency before deployment.

---

### 2I: Revised Section 6.3 Discussion - Sharded-N Allocation

Sharded-N remains consistently below baseline across the stable regime, consistent with persistent hot-path overhead. At Sharded-N's measured throughput of approximately 64,000 ops/s at 500 clients, the hot path incurs approximately 64,000 heap allocations per second. We hypothesize that allocator traffic and object lifecycle overhead dominate the expected sharding benefit in this configuration.

The similar about 3x overhead floor across GlobalMutex, Thread-Local, and Sharded-N may reflect a common hardware-level saturation ceiling rather than mechanistic equivalence; this is speculative.

---

### 2J: New Section 8.9 - Thread-Local Count-Based Flush Not Active Under v12 Workload Parameters

**8.9 Thread-Local Count-Based Flush Not Active Under v12 Workload Parameters**

The Thread-Local design defines two flush triggers: a count-based trigger every 1,000 records and a timer-based trigger every 100 ms (whichever occurs first). Under the present workload parameters, each client contributes 900 measured operations, and execution uses 8 worker threads. This yields approximately 900/8 = about 112.5 operations per thread per run.

Because about 112 << 1000, the 1,000-record count threshold does not fire during measured runs. Consequently, Thread-Local operates effectively in timer-driven flush mode throughout the experiment.

This matters for interpretation: the reported results characterize timer-only Thread-Local behavior, not the full hybrid design (count + timer) described in Section 3.3.4. Generalizability is therefore limited. In environments with sufficiently high per-thread work (at or above about 1,000 records per thread per run), both triggers would be active and performance could differ.

Future work should explicitly activate count-based flushing by increasing operations per client to at least 8,000 (so per-thread counts can cross 1,000), or by running at sustained throughput levels that repeatedly trigger count-based flushes during measurement.

---

### 2K: Revised Section 6.5 - Thread-Local (incorporating Section 8.9)

Thread-Local remains consistently below baseline in the current workload, indicating that deferred flush alone does not recover hot-path overhead under these conditions. Additionally, as discussed in Section 8.9, the workload parameters result in Thread-Local operating exclusively in timer-driven flush mode; the count-based threshold never fires. The results therefore characterize only one of the two designed flush modes. Interpretation should be limited accordingly: these findings do not establish behavior for configurations in which count-based flush activation is frequent.

---

### 2L: Revised Contributions List (Section 1)

1. We show that observability overhead in concurrent servers is strategy-dependent and regime-dependent, not intrinsically fixed.
2. We identify a bounded near-zero-overhead region for allocation-free sharded command-key aggregation (400-600 clients), with no significant baseline difference in primary comparisons within that range.
3. We demonstrate statistically significant divergence at 700 clients for the same strategy, establishing an upper-bound regime where overhead becomes measurable.
4. We characterize distinct degradation behaviors across alternative strategies, including threshold-like phase transition behavior and persistent under-baseline performance floors.

---

### 2M: Revised Section 6.6 - Exploratory vs Confirmatory Classification

This study distinguishes confirmatory from exploratory inference as follows.

CONFIRMATORY (pre-specified): The prediction that an allocation-free hot path would achieve near-zero overhead under the Universal Scalability Law's serialization model. Primary confirmatory tests are the baseline-vs-strategy comparisons at 500 clients with family-wise correction.

EXPLORATORY (post-hoc): Additional threshold-localized comparisons (for example, HdrHistogram-focused 400-client follow-up tests) introduced after initial pattern inspection. Bonferroni control is reported for transparency but is conservative in this setting; Benjamini-Hochberg false-discovery-rate control is the more appropriate correction for exploratory multi-test families.

Cross-family comparisons are treated as descriptive unless explicitly assigned to a prespecified hypothesis family.

---

### 2N: Revised Section 6.7 - Practical Implications (Scope Statement + recommendations)

The following recommendations are derived from a single experimental configuration: single-node Apple M2, Rust/Tokio runtime (8 worker threads), uniform key distribution over 10,000 keys, and loopback TCP benchmarking. They do not extend to distributed architectures, non-Rust runtimes, Zipfian key distributions, or server-class hardware with different core counts or memory hierarchies. Practitioners should treat these as hypotheses to validate in their own environment.

For high-throughput request-path metrics, prefer designs with allocation-free key handling and bounded key cardinality to minimize per-operation overhead.
When using sharded maps, ensure key space is well-distributed across shards; low-entropy or skewed keys can recreate shard hotspots and erase sharding gains.
Treat histogram-based shared update paths as potentially threshold-sensitive under concurrent writes, and validate concurrency transition points before production adoption.
Use staged rollout with workload-realistic benchmarks rather than extrapolating from single-regime results.

---

### 2O: New Section 7.6 - Alternative Concurrent Hashmaps

**7.6 Alternative Concurrent Hashmaps**

The flurry crate is a Rust adaptation of Java's ConcurrentHashMap design and uses epoch-based memory reclamation. Its synchronization and reclamation behavior differs materially from DashMap's per-shard RwLock structure, so contention behavior is not directly equivalent under write-heavy telemetry updates.

The evmap crate provides eventual-consistency semantics with lock-free reads and deferred writes. This model is attractive for read-heavy workloads but is less aligned with metrics paths that require timely and consistent aggregation of rapidly arriving updates.

The arc-swap crate provides atomic pointer replacement for shared immutable state. It is not a general concurrent hashmap and is not suitable for frequent per-key in-place counter mutation.

DashMap was selected in this study because it exposes RwLock-per-shard semantics directly, making the sharding contention model explicit and directly comparable to the GlobalMutex baseline. Empirical comparison against flurry or evmap for this workload is identified as future work.

[AUTHOR ACTION REQUIRED: Confirm this rationale for DashMap selection is accurate. If there was a different reason (e.g., ecosystem popularity, prior familiarity, dependency availability), replace with the actual reason. Accuracy matters here - reviewers will know if the justification is post-hoc.]

---

### 2P: Corrected Section 4.6 - Statistical Methodology (Full Replacement)

Primary inference was conducted on 30-run distributions per condition. Normality was evaluated using the Shapiro-Wilk test at alpha = 0.05 for each distribution prior to parametric testing. Distributions with SW p < 0.05 were analyzed with Mann-Whitney U rather than Welch's t-test. CV is reported as a stability descriptor only and was not used as a normality test criterion.

Three multiplicity families were defined:

Family 1 (confirmatory): throughput comparisons at 500 clients among the pre-specified strategy set, 8 pairwise tests, Bonferroni-corrected alpha = 0.05/8 = 0.00625 (reported as 0.006).
Family 2 (confirmatory): p99 comparisons at 500 clients using the same 8-test family definition and the same corrected alpha = 0.00625.
Family 3 (exploratory): HdrHistogram-focused post-hoc checks at 400 clients (2 tests), Bonferroni-corrected alpha = 0.05/2 = 0.025; this Bonferroni control is conservative for exploratory analysis, and Benjamini-Hochberg FDR is the preferred alternative for post-hoc multi-test settings.

Comparisons spanning different families are uncorrected and interpreted descriptively unless pre-specified. Effect sizes are reported for all primary comparisons: Cohen's d for parametric tests and rank-biserial correlation for Mann-Whitney tests.

---

### 2Q: Corrected Table 5.2.2 Footnote

The previous treatment of HdrHistogram was internally inconsistent: throughput at 500c (CV = 0.577) was excluded from Welch-family inference, while p99 at 500c (CV = 0.444) was retained despite the same non-normality concern. HdrHistogram p99 at 500c should be analyzed with Mann-Whitney U as well. Replace with: Mann-Whitney U = [TBD from raw data], one-tailed p < 0.001 (expected direction: HdrHistogram p99 > Disabled p99).

[AUTHOR ACTION REQUIRED - run Mann-Whitney on HdrHistogram p99 vs Disabled p99 at 500c raw data; replace placeholder.]

---

### 2R: Corrected Section 5.1 Run Count

At 500 clients, 9 of 30 runs were above 150,000 ops/s, 20 of 30 runs were below 100,000 ops/s, and 1 of 30 runs was between 100,000 and 150,000 ops/s.

[AUTHOR ACTION REQUIRED: Confirm that the 30th run falls in the 100,000-150,000 ops/s range. If not, correct accordingly from raw data.]

---

### 2S: Revised Section 5.1 Bimodality Causal Language

The 500-client data show a bimodal throughput pattern under sub-saturation load. This is reported as an observational distributional property only; alternative explanations, including TCP connection-establishment variation and Tokio scheduler initialization, were not investigated.

---

### 2T: Revised Section 5.3 Mann-Whitney Reporting

Mann-Whitney U = 886 (one-tailed, testing that HdrHistogram throughput < Disabled throughput), N1 = N2 = 30, p < 0.001, rank-biserial r = -0.969, indicating a very large directional effect (98.44% of pairwise comparisons in the hypothesized direction).

---

### 2U: Complete Revised Declarations Section

**Declarations**

**Ethics approval and consent to participate**
Not applicable. This study involves benchmark experiments on software systems and does not involve human participants or animals.

**Consent for publication**
Not applicable.

**Competing interests**
The author declares no competing interests.

**Funding**
No external funding was received for this work.

**Author contributions**
Single-author study. The author designed the experiments, implemented benchmarking and analysis code, executed experiments, analyzed results, and wrote the manuscript.

**Data and code availability**
Raw benchmark data (1,440 rows: 30 runs x 6 strategies x 8 concurrency levels), aggregated statistics, analysis scripts, and benchmark source code are archived at [REPOSITORY_URL], commit be472595525c361656ca88f3ed2d2861105083f0. A permanent archive with DOI [ZENODO_DOI] is deposited at Zenodo.

[AUTHOR ACTION REQUIRED: (1) Replace [REPOSITORY_URL] with your GitHub/GitLab repository URL. (2) Deposit data at https://zenodo.org and replace [ZENODO_DOI] with the resulting DOI before submission. Both are required for journal data availability compliance.]

---

### 2V: Literature Search Statement (for Section 1 and Section 7)

Passage for Section 1 (gap claim):
A search of proceedings from USENIX ATC, EuroSys, VLDB, SIGMETRICS, ACM SOSP, and IEEE ICDCS (2015-2025) yielded no controlled multi-strategy study measuring telemetry overhead as the primary variable across concurrency levels in a server context [AUTHOR: confirm search or replace with your actual search scope and result].

Passage for Section 7.2 (novelty claim):
To our knowledge, based on a search of [VENUE LIST], no prior study has characterized HdrHistogram's performance collapse threshold under concurrent server load; our results constitute the first such controlled characterization [AUTHOR: confirm search scope or revise if prior work exists].

[AUTHOR ACTION REQUIRED: Confirm or correct the venue list searched, and confirm no prior work was found. If a prior work characterizing HdrHistogram collapse exists, cite it and revise the novelty claim accordingly.]

## 3. Structural Items (3A-3D)

### 3A: Complete Table 5.4.3 - Low Concurrency Data (100-300 clients)

Table 5.4.3: Throughput (ops/s) at low concurrency (100-300 clients). These concurrency levels are outside the primary stable regime; CV is high and parametric tests are not applied. Values are means reported for descriptive purposes. Confidence intervals are not available for these cells; see raw data archive [ZENODO_DOI] for per-run values.

| Strategy | c=100 | c=200 | c=300 |
|---|---:|---:|---:|
| Disabled | 181,803 | 130,381 | 180,629 |
| GlobalMutex | 206,150 | 211,687 | 213,691 |
| Sharded-2key | [AUTHOR: report] | [AUTHOR: report] | [AUTHOR: report] |
| HdrHistogram | [AUTHOR: report] | [AUTHOR: report] | [AUTHOR: report] |
| ThreadLocal | [AUTHOR: report] | [AUTHOR: report] | [AUTHOR: report] |
| Sharded-N | [AUTHOR: report] | [AUTHOR: report] | [AUTHOR: report] |

[AUTHOR ACTION REQUIRED: Fill in means for Sharded-2key, HdrHistogram, ThreadLocal, and Sharded-N at 100, 200, and 300 clients from raw data. These values are referenced in Section 6.4 and should appear in this table rather than only in the Discussion.]

---

### 3B: Anomaly Decision Inline Text

The formal outlier decision for this run is recorded as follows: threshold = 3x median; v10 anomaly run value = 151,557 ops/s, v10 median about 31,779 ops/s, ratio about 4.77 (exceeds threshold); however, the anomaly does not appear in the v12 dataset. In v12, Sharded-2key run 1 at 500 clients = 211,579 ops/s vs. median = 217,618 ops/s (ratio = 0.97, well within threshold). A dedicated rerun produced run 1 = 48,785 ops/s vs. median = 60,702 ops/s (ratio = 0.80, within threshold). Formal decision: no_formal_outlier; no runs excluded. Full per-run values and the automated detection script are available at [ZENODO_DOI].

---

### 3C: Appendix A Stub - Sharded-2key Code Reference

Appendix A: Sharded-2key Collector Implementation

The Sharded2KeyCollector struct and record() method are reproduced below for reference. The full implementation is available in the archived repository at commit be472595, src/command_metrics.rs, lines 180-199.

```rust
struct Sharded2KeyCollector {
    data: DashMap<&'static str, CommandStat>,
}

impl Sharded2KeyCollector {
    fn record(&self, cmd_name: &'static str, duration_us: u64) {
        self.data
            .entry(cmd_name)
            .and_modify(|stat| stat.record(duration_us))
            .or_insert_with(|| {
                let mut s = CommandStat::new();
                s.record(duration_us);
                s
            });
    }
}
```

The key property is that cmd_name is typed as &'static str - a compile-time string literal - which eliminates heap allocation on the hot path. All other configuration (DashMap::new(), 64 shards, ahash hasher) is identical to the Sharded-N implementation.

---

### 3D: Figure Quality Checklist

Figure Publication Readiness Checklist (complete before submission):

Figure 1 (Throughput vs. Concurrency):
- [ ] Axes labeled with units: x-axis Concurrent Clients, y-axis Throughput (ops/s)
- [ ] Y-axis scale starts at 0 or clearly noted if not
- [ ] 95% CI error bars visible and labeled in legend
- [ ] Six strategies distinguished by both color AND line style (colorblind-safe palette: e.g., Okabe-Ito)
- [ ] Legend does not overlap data
- [ ] Caption does not duplicate body text

Figure 2 (p99 vs. Concurrency):
- [ ] Axes labeled with units: x-axis Concurrent Clients, y-axis p99 Latency (us)
- [ ] Y-axis scale starts at 0 or clearly noted if not
- [ ] 95% CI error bars visible and labeled in legend
- [ ] Six strategies distinguished by both color AND line style (colorblind-safe palette: e.g., Okabe-Ito)
- [ ] Legend does not overlap data
- [ ] Caption does not duplicate body text

Figure 3 (Distribution at 500c - boxplot or violin):
- [ ] Axes labeled with units and metric definition
- [ ] Distribution geometry (box/violin) clearly legible at publication size
- [ ] Outlier markers and median/quantile markers are visible
- [ ] Strategy encoding uses both color and pattern/line treatment
- [ ] Legend does not overlap data
- [ ] Caption states sample size and plot type

Figure 4 (CV vs. Concurrency):
- [ ] Axes labeled with units: x-axis Concurrent Clients, y-axis Coefficient of Variation
- [ ] Y-axis scale and any threshold lines clearly labeled
- [ ] Six strategies distinguished by both color AND line style (colorblind-safe palette)
- [ ] Legend does not overlap data
- [ ] Caption does not duplicate body text

[AUTHOR ACTION REQUIRED: Open each figure in your plotting tool. Verify every checkbox. Regenerate any figure that fails. Export at minimum 300 DPI for journal submission.]

## 4. Reference Upgrades (Items 1-6)

1. [2] Dapper (upgrade)
   - Sigelman, B. H., Barroso, L. A., Burrows, M., Stephenson, P., Plakal, M., Beaver, D., Jaspan, S., & Shanbhag, C. (2010). Dapper, a Large-Scale Distributed Systems Tracing Infrastructure. Google Technical Report.
   - URL: https://research.google/pubs/pub36356/
   - PDF: http://research.google.com/archive/papers/dapper-2010-1.pdf
   - Accessed: 2026-04-21

2. [6] OpenTelemetry (OSDI/EuroSys/USENIX check + archive fallback)
   - No OSDI, EuroSys, or USENIX architecture/performance paper for OpenTelemetry was identified in this package build.
   - Use archived, versioned documentation citation:
   - OpenTelemetry Authors. (n.d.). OpenTelemetry Documentation.
   - Archived snapshot: https://web.archive.org/web/20260409133655/https://opentelemetry.io/docs/
   - Canonical URL: https://opentelemetry.io/docs/
   - Accessed: 2026-04-21

3. [13] Prometheus (upgrade)
   - Preferred upgrade:
   - Beyer, B., Murphy, N. R., Jones, C., & Petoff, J. (Eds.). (2016). Site Reliability Engineering: How Google Runs Production Systems. O'Reilly Media. ISBN 9781491929117.
   - URL: https://www.oreilly.com/library/view/site-reliability-engineering/9781491929117/
   - Accessed: 2026-04-21
   - Optional archival support for project documentation:
   - https://web.archive.org/web/20260412123848/https://prometheus.io/docs/introduction/overview/

4. [23] HdrHistogram (upgrade)
   - Tene, G. (2013, April 4). How NOT to Measure Latency (QCon London 2013 presentation). InfoQ.
   - URL: https://www.infoq.com/presentations/latency-pitfalls/
   - Accessed: 2026-04-21
   - Supplemental software source:
   - HdrHistogram project page: https://hdrhistogram.github.io/HdrHistogram/

5. [15] Tokio (software citation)
   - Tokio Contributors (tokio-rs). (2025). Tokio (Version 1.48.0) [Software].
   - Crate URL: https://crates.io/crates/tokio/1.48.0
   - Source repository: https://github.com/tokio-rs/tokio
   - Accessed: 2026-04-21

6. [19] StatsD (software citation)
   - StatsD Contributors; originally developed at Etsy. (2011-present). StatsD [Software].
   - URL: https://github.com/statsd/statsd
   - Accessed: 2026-04-21

## 5. Central Claim (Definitive Revised Version)

Sharded-2key achieves throughput and p99 latency statistically indistinguishable from the disabled baseline at 400-600 clients (throughput p = 0.086-0.107, p99 p = 0.15-0.628; all ns after Bonferroni). At 700 clients, a statistically significant throughput difference of 6,207 ops/s (2.85%; p = 0.000402) and p99 difference of 986 us (13.26%; p = 0.003864) emerges, suggesting a bounded overhead effect at the upper boundary of the stable regime.

## 6. Author Action Summary (Consolidated)

1. Run Mann-Whitney on raw 30-run p99 values for HdrHistogram vs Disabled at 500c; replace placeholder U and exact p-value in Section 5.2.2/Section 5.3 text.
2. Run Shapiro-Wilk on each 30-run distribution for all strategies at 400/500/600/700c; populate SW table and update test selections in Tables 5.2.1 and 5.2.2.
3. Confirm Section 3.3.3 causal explanation (workload-parameterization vs code-structure change) is fully accurate; if additional code changed, document exact before/after and mechanism.
4. Confirm DashMap-selection rationale in new Section 7.6; replace with true historical rationale if different.
5. Confirm the 30th run category in corrected Section 5.1 run-count sentence (100k-150k bin) from raw data and correct if needed.
6. Replace [REPOSITORY_URL] with actual repository URL in Declarations.
7. Deposit the dataset and scripts on Zenodo and replace [ZENODO_DOI] everywhere before submission.
8. Confirm or correct literature-search scope and venue list in Section 1 and Section 7 statements; revise novelty claim if prior work exists.
9. Fill missing low-concurrency means (100/200/300c) for Sharded-2key, HdrHistogram, ThreadLocal, and Sharded-N in Table 5.4.3 from raw data.
10. Complete the figure publication checklist for Figures 1-4, regenerate any failing figure, and export at minimum 300 DPI.

All items in this Data Package are computed from the provided dataset or clearly flagged as requiring author input. No values have been fabricated.
