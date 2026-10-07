# Cover letter — Software: Practice and Experience

Saksham Kapoor
Independent Researcher, Delhi, India
sakshamkapoor810@gmail.com

To the Editors, *Software: Practice and Experience*

Dear Editors,

I am submitting the manuscript **"Cardinality Before Contention: A Factorial
Decomposition of Per-Command Observability Overhead in a Concurrent In-Memory
Key-Value Store"** for consideration as a research article.

**What the paper finds.** Practitioner guidance on instrumenting a server's
request path is close to unanimous: keep the hot path lock-free, shard the
counter map, replace the mutex. The paper shows that on a thread-pool
concurrent server this advice optimizes the smaller of two costs. Rather than
ranking metrics strategies — a ranking tells an engineer nothing about which
decision inside a strategy to revisit — I instrument a Redis-compatible
in-memory key-value server in Rust with a set of per-command collectors chosen
so that adjacent pairs differ along exactly one design axis, which turns the
comparison into a factorial decomposition and prices each axis separately. Four
results follow:

1. Moving the metric key from a 2-entry command-name label to a 10,000-entry
   data-key label costs 1.66× what replacing a global mutex with a 64-way
   sharded concurrent map saves. The consequence is an inversion of the usual
   priority: the sharded "scalable" map carrying a high-cardinality label is
   measurably slower than a single global lock carrying a low-cardinality one.
2. The axes obey different scaling laws. No axis responds to a 30× increase in
   client concurrency on a demonstrably saturated server, while the
   synchronization axis tracks worker-thread count instead — so the knob
   benchmarks conventionally turn cannot, even in principle, expose telemetry
   contention on this architecture.
3. Overhead is not one number: counter costs are elastic and dissipate into
   queueing slack at the tail, whereas histogram-payload cost is rigid and
   arrives undiminished at p99. This produces a sign reversal between p50 and
   p99 that a throughput-only evaluation reports as a confident null.
4. A purpose-built collector whose per-operation work is identical at every
   cardinality establishes that the effect is the live entry count itself
   rather than the key allocation bundled with it, and bounds the claim: below
   roughly 1,000 entries the effect is indistinguishable from zero.

**Fit with the journal's aims and scope.** *Software: Practice and Experience*
publishes work on the practical experience of building and measuring software
systems, including performance evaluation, measurement methodology and
experience with specific implementations. This paper is squarely of that kind.
Its subject is a concrete, working system; its result is a design rule that
changes what an engineer should do first when instrumenting a server; and its
methodological contribution — pricing design axes rather than strategy bundles,
reporting against more than one budget metric, and clustering inference on
independent cloud instances rather than on correlated blocks within them — is
offered as transferable practice rather than as theory.

**Evidence and reproducibility.** The results rest on 29,520 benchmark runs
collected with Randomized Multiple Interleaved Trials across fourteen Azure
virtual machines provisioned solely for the experiment, in three regions. All
raw data, the experiment runner, the analysis code and the figure code are
public, and Appendix A gives the commands that regenerate every table, figure
and quoted statistic in the paper from the committed data. The artifact is
archived with a persistent identifier; the data-availability statement gives
the DOI and the repository URL.

**Declarations.** This manuscript is original work, has not been published
previously, and is not under consideration at any other journal. There are no
competing interests to declare and the work received no funding. A declaration
of generative-AI use is included in the manuscript: AI tools assisted with code
authoring, drafting and literature search, and were not used to generate,
simulate or alter any measurement. I have selected the subscription publication
option.

Thank you for considering the manuscript.

Yours sincerely,

Saksham Kapoor
