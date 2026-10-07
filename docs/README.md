# Docs Index

The submitted manuscript:

- [final_paper_v27.md](final_paper_v27.md): **"Cardinality Before
  Contention"** — decomposes per-command observability overhead into four
  design axes (synchronization, cardinality, aggregation, payload) and shows
  the cardinality axis dominates the synchronization axis, that the axes scale
  with worker threads rather than client load, and that histogram cost alone
  survives into the tail. Also available as `final_paper_v28.docx`. Every
  table, figure and quoted statistic regenerates from the committed data; see
  the paper's Appendix A.

Supporting documents:

- [rmit_experiment_protocol.md](rmit_experiment_protocol.md): how the main
  experiments are run, including Azure VM provisioning
- [followup_experiment_protocol.md](followup_experiment_protocol.md): the
  axis-separation batch — splits the two confounded design axes using the
  `sharded_bucketed` and `thread_local_owned` collectors
- [system-design.md](system-design.md): server architecture
- [failure-analysis.md](failure-analysis.md): failure modes
- [related_work_notes.md](related_work_notes.md): related-work notes
- [figures/](figures): paper figures, in PNG and vector PDF

Superseded material lives in [`../archive/`](../archive). `archive/paper_draft.md`
is an earlier, longer measurement-study write-up of the same dataset whose
content is now folded into `final_paper_v27.md`. It is kept for provenance
only; it is **not** a companion or related publication, has never been
submitted anywhere, and the manuscript does not cite it.
