# Annotated related-work notes

Moved out of `docs/paper_draft.md` §1 (which is now a condensed related-work
section). Retained as working notes; some early annotations below predate the
full read of Laaber et al. (2019) and the concept-note cross-check — where they
conflict with the paper, the paper is authoritative.

## Original annotated bibliography

Observability (per-command metrics/counters) is not free; the
implementation strategy for collecting it can dominate or disappear into
measurement noise depending on concurrency. This project asks two
questions that turn out to be almost independent of each other: (1) how
much does per-command metrics collection cost in a concurrent in-memory
key-value store, and which collection strategy costs least, and (2) can a
benchmark answer question (1) without a confound as large as the effect
it's trying to measure — which, as §2 shows, the project's own first
attempt could not.

**On question (2) — benchmark methodology — closely related prior work
exists, and one paper in particular already does most of what this
paper's §3 presents as a "fix."** This needs to be stated plainly rather
than glossed over:

- **Abedi, A., Heard, C., and Brecht, T. (2015). "Conducting Repeatable
  Experiments and Fair Comparisons using 802.11n MIMO Networks." ACM
  SIGOPS Operating Systems Review, 49(1)** proposed **Randomized Multiple
  Interleaved Trials (RMIT)** for WiFi experiments, and
  **Abedi, A. and Brecht, T. (2017). "Conducting Repeatable Experiments
  in Highly Variable Cloud Computing Environments." ICPE '17** showed it
  is needed, not optional, in cloud environments. RMIT interleaves
  randomly-ordered trials of the configurations under comparison (one
  fresh random order per round) so that time-varying environmental
  conditions land on every configuration roughly equally rather than
  confounding with one of them; the 2017 paper demonstrates on
  pre-collected EC2 benchmark traces that single-trial and
  multiple-*consecutive*-trial designs falsely report differences of up to
  37.8% between two identical systems, and that even non-randomized
  interleaving (MIT) fails when the environment changes periodically.
  **This is the same technique, under the same name, that this paper's §3
  presents; the technique, its name, and its core justification are
  Abedi, Heard, and Brecht's contribution, not this project's.** What
  this paper adds on top is scoped to §3-§7's specific application:
  applying RMIT to live per-command observability-strategy comparison in
  a Rust in-memory key-value store (their 2017 study ran no live
  experiments — it replays traces collected by others), the
  file-descriptor exhaustion failure mode in §3 (a systematic,
  order-independent failure their design doesn't discuss), and the
  explicit minimum-detectable-effect analysis in §7 (their paper
  establishes RMIT's validity but does not report a power analysis for a
  specific comparison).
- **Laaber, C., Scheuner, J., and Leitner, P. (2019). "Software
  Microbenchmarking in the Cloud. How Bad is it Really?" Empirical
  Software Engineering, 24(4), 2469-2508.** This is the closest overall
  prior work to this paper's actual method: it applies RMIT plus
  bootstrap-based statistical comparison (Wilcoxon rank-sum and
  overlapping confidence intervals) to detect performance slowdowns in
  real open-source Java/Go microbenchmark suites run on public cloud
  instances (AWS, GCE, Azure) against a bare-metal baseline, and reports
  that this combination reliably detects slowdowns as small as ~10% with
  20 instances. The design (RMIT + bootstrap CIs + cloud VMs +
  quantifying detectable-effect size) is structurally the same recipe
  this paper follows in §3, §4, and §7. The differences are the unit
  under test (fine-grained library microbenchmarks across many unrelated
  OSS projects vs. one server's own six instrumentation strategies under
  realistic concurrent client load) and the experimental unit: this
  paper's minimum detectable effect (roughly 1-2%, §7) comes from 15-30
  within-machine repetitions of one fast workload, whereas their ~10%
  figure comes from cross-instance comparisons of benchmarks whose own
  variability ranges from 0.03% to over 100% CV. The two numbers answer
  different questions and are not a like-for-like comparison.
- **Mytkowicz, T., Diwan, A., Hauswirth, M., and Sweeney, P. F. (2009).
  "Producing Wrong Data Without Doing Anything Obviously Wrong!" ASPLOS
  '09.** Documents that unrandomized, seemingly innocuous aspects of an
  experimental setup (environment size, link order) silently bias
  measured performance across widely-used architectures and compilers,
  and that most published systems papers surveyed do not control for it.
  Establishes that §2's failure mode (an unrandomized design producing a
  wrong or unsupportable conclusion) is a documented, common pattern, not
  a one-off mistake specific to this project.
- **Georges, A., Buytaert, D., and Eeckhout, L. (2007). "Statistically
  Rigorous Java Performance Evaluation." OOPSLA '07.** Establishes
  reporting confidence intervals over many iterations, rather than a
  single mean, as the standard for managed-runtime performance
  evaluation. Basis for this project's move (§3, §6) from mean ± stddev
  to bootstrap confidence intervals on the median.
- **Kalibera, T. and Jones, R. E. (2013). "Rigorous Benchmarking in
  Reasonable Time." ISMM '13.** Gives a principled method for choosing
  how many repetitions a benchmark needs under a fixed time budget.
  Directly relevant to this project's own repetition-count choices
  (15 vs. 30 per cell across the three datasets) and to the power
  analysis in §7, which is this paper's attempt at exactly the kind of
  "is this enough repetitions" question Kalibera and Jones formalize.
- **Curtsinger, C. and Berger, E. D. (2013). "STABILIZER: Statistically
  Sound Performance Evaluation." ASPLOS '13.** A complementary
  randomization approach: rather than randomizing *when* configurations
  run (RMIT's approach, and this project's), STABILIZER randomizes
  *where in memory* code and data land, to remove layout-driven
  measurement bias from a single run. Cited here because it establishes
  that "randomize an experimental factor that shouldn't matter but
  secretly does" is a broader, recurring pattern in systems performance
  evaluation, of which the run-order confound this project hit is one
  instance among several documented in the literature.
- **David, T., Guerraoui, R., and Trigonakis, V. (2013). "Everything You
  Always Wanted to Know About Synchronization but Were Afraid to Ask."
  SOSP '13.** Broad empirical study of lock and atomic-operation costs
  across real hardware; background for why a global mutex, sharded
  counters, and thread-local accumulation could plausibly differ in cost
  under contention, motivating this project's choice of exactly those
  strategies as the comparison set.
- **Aspnes, J., Herlihy, M., and Shavit, N. (1994). "Counting Networks."
  Journal of the ACM, 41(5), 1020-1048.** Foundational work on
  low-contention concurrent counting via structured networks rather than
  a single shared counter; the theoretical ancestor of this project's
  sharded-counter strategies (`sharded_2key`, `sharded_n`).
- **Boyd-Wickizer, S., Clements, A. T., Mao, Y., Pesterev, A., Kaashoek,
  M. F., Morris, R., and Zeldovich, N. (2010). "An Analysis of Linux
  Scalability to Many Cores." OSDI '10.** Introduces "sloppy counters"
  (per-core counters reconciled periodically, rather than a single
  contended shared counter) as a practical fix for kernel counter
  contention on many-core Linux. The closest existing practical analogue
  to this project's `thread_local` strategy, and consistent with this
  paper's own finding (§4b) that `thread_local` is the cheapest strategy
  in every dataset tested.
- **Tallent, N. R., Mellor-Crummey, J., and Porterfield, A. (2010).
  "Analyzing Lock Contention in Multithreaded Applications." PPoPP '10.**
  Addresses the observer-effect problem directly: a measurement tool can
  itself perturb the lock contention it is trying to measure. Relevant
  motivation for why this project treats "does the metrics-collection
  strategy itself add overhead" as a first-class, separately-measured
  question (§4's overhead tables) rather than assuming instrumentation is
  free.
- **Sigelman, B. H. et al. (2010). "Dapper, a Large-Scale Distributed
  Systems Tracing Infrastructure." Google Technical Report.** Industrial
  evidence that per-request observability overhead is a real, actively
  managed cost at scale — Google bounds Dapper's overhead to a small
  fraction of one CPU core per machine via sampling specifically because
  uncontrolled tracing overhead was judged unacceptable in production.
  Motivates why precisely quantifying instrumentation overhead, as this
  paper does, is a practically important question and not only an
  academic one.
- **Tene, G. "HdrHistogram: A High Dynamic Range Histogram."**
  (Software, public domain.) The library underlying this project's
  `hdr_histogram` metrics strategy; included for completeness since it is
  referenced by name throughout §3-§7, not because it is a peer-reviewed
  source.
- **Fruth, M., Scherzinger, S., Mauerer, W., and Ramsauer, R. (2021).
  "Tell-Tale Tail Latencies: Pitfalls and Perils in Database
  Benchmarking." TPCTC 2021.** The closest database-benchmarking-specific
  methodology paper found in this search: shows that Java-based database
  benchmarking harnesses (as used by standard benchmarks like TPC-X and
  YCSB) systematically distort measured tail latencies, a different
  confound than this paper's run-order problem (§2) but the same
  underlying lesson — a benchmark harness's own implementation details
  can be the dominant source of a measured effect, not the system under
  test. Cited here specifically because it is KV-store/database-adjacent
  benchmarking-methodology literature, which §7 notes this paper's
  related-work search otherwise found little of.
- **Cooper, B. F., Silberstein, A., Tam, E., Ramakrishnan, R., and Sears,
  R. (2010). "Benchmarking Cloud Serving Systems with YCSB." SoCC '10.**
  Defines the Yahoo! Cloud Serving Benchmark (YCSB), the de facto
  standard tool for comparing NoSQL/key-value-store throughput and
  latency under configurable CRUD workload mixes. Cited here specifically
  to note a limitation directly: this project's own benchmark client
  (`benchmarks/`) is custom-built rather than YCSB, so its mixed/
  read-heavy/write-heavy workloads (§3, §4b) are conceptually similar to
  YCSB's workload mixes but are not the standardized, externally
  validated YCSB workloads themselves (see §7).
- **Reniers, V., Van Landuyt, D., Rafique, A., and Joosen, W. (2017). "On
  the State of NoSQL Benchmarks." ICPE '17 Companion.** A direct
  methodology critique of NoSQL/KV-store benchmarking practice, arguing
  that many published comparisons under-report workload configuration,
  hardware variability, and statistical treatment of results. The closest
  match found to a KV-store-specific analogue of this paper's own §2
  critique of the v12 design, though focused on cross-study comparability
  problems rather than the within-study run-order confound this paper
  addresses.
- **Fan, B., Andersen, D. G., and Kaminsky, M. (2013). "MemC3:
  Compact and Concurrent MemCache with Dumber Caching and Smarter
  Hashing." NSDI '13.** Replaces Memcached's global-lock-protected hash
  table with a concurrent, mostly-lock-free cuckoo hash table, achieving
  up to 3x the query throughput. Relevant background for the general
  claim behind this project's `sharded_2key`/`sharded_n` strategies —
  that replacing a single global lock with a more concurrency-friendly
  data structure can remove a throughput bottleneck in a KV-store-class
  system — even though this paper's own results (§4, §4b) find that
  benefit does not clearly materialize for command-metrics counters at
  the concurrency levels tested (4-8 vCPUs, up to 3000 clients).

**What this leaves for this paper to contribute**, given the above: not
the RMIT technique itself (Abedi and Brecht, 2017), not the general
practice of randomized-order or bootstrap-based benchmarking (also
Laaber et al., 2019; Georges et al., 2007), but (a) a specific,
previously-unpublished empirical answer — the actual overhead of six
named counter/histogram strategies in a Rust in-memory key-value store,
replicated across three independent machines and up to 3000 concurrent
clients, (b) the file-descriptor exhaustion failure mode in §3, which is
a new, concrete instance of the "clean but wrong" measurement pathology
Mytkowicz et al. describe in the abstract but which RMIT itself does not
catch, and (c) an explicit power analysis (§7) quantifying exactly what
effect sizes this specific experiment could and could not have detected,
which neither Abedi and Brecht (2017) nor Laaber et al. (2019) report for
their own comparisons.



## Additional papers added after the concept-note cross-check

- **Bulej, L., Horky, V., Tuma, P., Farquet, F., and Prokopec, A. (2020).
  "Duet Benchmarking: Improving Measurement Accuracy in the Cloud." ICPE
  '20.** Runs two artifacts *in parallel* on the same machine so shared
  noise affects both equally; reports accuracy improvements of 2.3x-12.5x
  (ScalaBench/DaCapo) and 23.8x-82.4x (SPEC CPU 2017). A paired design like
  this paper's, but simultaneous rather than interleaved; unsuitable when
  the two artifacts would contend for the same saturated resource (two
  server instances on one host).
- **Maricq, A., Duplyakin, D., Jimenez, I., Maltzahn, C., Stutsman, R.,
  and Ricci, R. (2018). "Taming Performance Variability." OSDI '18.**
  Nearly 900,000 data points from 835 servers over 10 months; shows how
  performance varies across repeated runs and across identical servers, and
  provides a statistical model and tool for choosing experiment parameters.
- **Uta, A. et al. (2020). "Is Big Data Performance Reproducible in Modern
  Cloud Networks?" NSDI '20.** Millions of data points from 9+ PB of
  network transfers on commercial and private clouds; network variability
  persists and QoS mechanisms can intensify it; big-data workloads show
  significant slowdowns and poor predictability; community often neglects
  variability.

## Notes on Laaber et al. (2019) after a full read

19 microbenchmarks (20 planned, one lost to a configuration error) from
Log4j2, RxJava, bleve, and etcd; AWS/GCE/Azure x 3 instance types plus IBM
Bluemix bare metal; 50 instances x 10 trials x 50 iterations, >4.5M data
points; result CV from 0.03% to 100.68%. A/A tests (100 random splits; 5%
false-positive threshold) and injected-slowdown minimal-detectable-slowdown
analysis (0.1%-1000%, 95% true-positive rate). Trial-based sampling (same
instances, randomized order) needs far fewer samples than instance-based;
five instances x five trials detect slowdowns in the 5%-10% range for most
benchmarks. Threats to validity: results are for microbenchmarks only; no
claims for stress or load tests; lock contention listed as future work.
