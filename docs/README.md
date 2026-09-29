# Docs Index

Two papers are written from the same experimental data, answering different
questions:

- [paper_design_axes.md](paper_design_axes.md): **"Cardinality Before
  Contention"** — decomposes observability overhead into four design axes
  (synchronization, cardinality, aggregation, payload) and shows the
  cardinality axis dominates the synchronization axis, that the axes scale with
  worker threads rather than client load, and that histogram cost alone
  survives into the tail. This is the paper with the novel claim.
- [paper_draft.md](paper_draft.md): the underlying RMIT measurement study —
  overhead magnitudes, the strategy ranking, replication across eleven runs,
  and the detectable-effect analysis.

Supporting documents:

- [rmit_experiment_protocol.md](rmit_experiment_protocol.md): how the experiments are run
- [followup_experiment_protocol.md](followup_experiment_protocol.md): the next
  batch — splits the two confounded design axes using the `sharded_bucketed` and
  `thread_local_owned` collectors
- [system-design.md](system-design.md): server architecture
- [failure-analysis.md](failure-analysis.md): failure modes
- [related_work_notes.md](related_work_notes.md): related-work notes
