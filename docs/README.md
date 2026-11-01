# Docs Index

The submitted manuscript:

- [final_paper_v27.md](final_paper_v27.md): **"Cardinality Before
  Contention"** — decomposes per-command observability overhead into four
  design axes (synchronization, cardinality, aggregation, payload) and shows
  the cardinality axis dominates the synchronization axis, that the axes scale
  with worker threads rather than client load, and that histogram cost alone
  survives into the tail. Measured over 29,520 RMIT runs on fourteen dedicated
  Azure VMs, with inference clustered on independent runs. Submitted to
  *Software: Practice and Experience*; this Markdown source is the manuscript of
  record and matches what was submitted section for section, table for table
  (Tables 1–14) and number for number — only the typesetting differed. Every
  table, figure and quoted statistic regenerates from the committed data; see
  the paper's Appendix A. The tagged version is `v1.0-paper`, archived at
  [doi:10.5281/zenodo.23295261](https://doi.org/10.5281/zenodo.23295261).

Supporting documents:

- [rmit_experiment_protocol.md](rmit_experiment_protocol.md): how the main
  experiments are run, including Azure VM provisioning
- [followup_experiment_protocol.md](followup_experiment_protocol.md): the
  axis-separation batch — splits the two confounded design axes using the
  `sharded_bucketed` and `thread_local_owned` collectors
- [system-design.md](system-design.md): server architecture. Predates the
  manuscript and reports its own earlier storage-backend and AOF benchmarks;
  for anything about observability-overhead magnitudes the manuscript is
  authoritative
- [failure-analysis.md](failure-analysis.md): crash-recovery, partial-write and
  contention failure modes of the server — orthogonal to the manuscript's
  measurements, except for the open-FD failure the paper records in §4.4
- [related_work_notes.md](related_work_notes.md): related-work notes
- [figures/](figures): paper figures, in PNG and vector PDF

Superseded material lives in [`../archive/`](../archive). `archive/paper_draft.md`
is an earlier, longer measurement-study write-up of the same dataset whose
content is now folded into `final_paper_v27.md`. It is kept for provenance
only; it is **not** a companion or related publication, has never been
submitted anywhere, and the manuscript does not cite it.
