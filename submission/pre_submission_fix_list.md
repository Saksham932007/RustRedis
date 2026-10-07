# RustRedis paper: pre-submission fix list

Oct 7, 2026 · @Saksham Kapoor

**Status as of Oct 7, 2026.** Sections 1 and 2 are complete. Section 3 is
complete except for the two items that require new experiments, which were
explicitly out of scope (no further runs); both are now stated in §7 as the
experiments that would settle the corresponding limitations. Section 4 is
complete except for the items that need an account, a website form or a payment
choice — those are listed in `submission/SUBMISSION_CHECKLIST.md` with
instructions.

Section numbers (§2, §5.2 …) refer to `docs/final_paper_v27.md`.

## 1. GitHub repo (do first)

- [x] **Rewrite the README around the paper.** Already done in commit `40c8d5b`
      ("Rewrite README around the axis-decomposition results"); this pass added
      the dependency-install step, the submission/archive pointers, a "Citing
      this work" section, the vector-PDF note, and removed the references to
      the superseded draft.
- [x] **Make every Appendix A path real.** All seven scripts and all three
      `experiments/azure_*` trees are at the paths the paper names; verified
      from a fresh clone. Appendix A was also corrected to name both JSON
      outputs and the PNG/PDF figure outputs.
- [x] **Add the two protocol docs.** `docs/rmit_experiment_protocol.md` and
      `docs/followup_experiment_protocol.md` are both present and cited.
- [x] **Test the reproduction on a fresh clone.** Every Appendix A command was
      run against a fresh `git clone`; all eight scripts exit 0 and the outputs
      are byte-identical to the committed artifacts, so a successful
      reproduction leaves `git status` clean. Log:
      `submission/reproduction_log.md`. One real bug was found and fixed in the
      process: `analyze_design_axes.py --followup-only` wrote to the same JSON
      path as the full run, so running Appendix A's commands in the documented
      order clobbered the main analysis file. It now writes
      `design_axes_followup_analysis.json`. Figure PDFs were also made
      byte-deterministic.
- [x] **Move old material into an `archive/` folder.** The specific files named
      here (`results/final_experiment_v5`, `experiment_results_v12`,
      `v15_evidence.json`, `final_experiment_v12.json`, the old reports, the
      macOS docs, and `appendonly.aof`) had already been removed in commit
      `6677eb1` ("Clean and standardize repo layout; keep only Azure VM data")
      and are not in the tree or in `git ls-files`; `*.aof` is in
      `.gitignore`. What remained to archive was the superseded draft and its
      figures, now in `archive/` with a README explaining their status.
- [x] **Decide what `docs/paper_draft.md` is.** Resolved as *not* a related
      paper: moved to `archive/paper_draft.md` and labelled, in both
      `archive/README.md` and `docs/README.md`, as superseded working material
      that was never submitted anywhere. The manuscript no longer references it
      (see §2 below), so there is nothing to disclose to the editor.
- [x] **Add a repo description, topics and a `CITATION.cff` file.** Description
      and twelve topics set on GitHub; `CITATION.cff` added at the root with
      both the software entry and a `preferred-citation` for the paper (ORCID
      and DOI lines present but commented out until they exist).
- [~] **Tag a release and archive it on Zenodo.** Tag `v1.0-paper` created
      locally. Pushing the tag, publishing the GitHub release, and creating the
      Zenodo deposition need the author's accounts — see
      `submission/SUBMISSION_CHECKLIST.md`. Every place the DOI must be pasted
      is marked with `10.5281/zenodo.XXXXXXX`.

## 2. Paper: must-do fixes

- [x] **Cut the abstract from about 540 words to 200–250.** Now 250 words: one
      block for problem and design, one sentence per finding, one for the
      takeaway. Run counts, intervals and p-values moved into the body.
- [x] **Add citations to unsupported claims.**
  - §2 instrumentation overhead: the vague "tens of percent / roughly 8%"
    sentence is replaced with cited measurements — Nõu et al. [31] (19–80%
    throughput reduction, up to 175% latency, OpenTelemetry and Elastic APM),
    Reichelt et al. [36] (MooBench: OpenTelemetry vs inspectIT vs Kieker) and
    Reichelt et al. [37] (up to 30% spread between instrumentation
    technologies), plus Pin [25] and Canopy [19].
  - §2 cardinality as a storage and billing cost: Gorilla [34], Monarch [3],
    the observability-data-management survey [21], the Prometheus naming
    guidance [35], Brazil [8] and the OpenTelemetry specification's cardinality
    limits [32].
  - §1 "keep the hot path lock-free": the Prometheus Go client's lock-free
    histogram observation path [5], Brazil [8], Gregg [17] and Bos [6].
- [x] **Cite reference [19] (HdrHistogram) in the text.** §3.2 now introduces
      the `hdr_histogram` collector with a paragraph citing Tene's HdrHistogram
      [42, formerly 19] and the Rust port [18].
- [x] **Grow the bibliography from 20 to roughly 35–50 references.** Now **46**,
      alphabetical, with every in-text citation renumbered. Added Tokio [43],
      DashMap [46], the Rust HdrHistogram port [18], the Rust `prometheus` [39]
      and `metrics` [28] crates, the instrumentation-overhead studies above,
      cardinality/time-series work [3, 21, 34, 35, 32], concurrency work
      [24, 27, 30], tail latency [13], and cloud-benchmarking methodology
      [23, 33]. Checked mechanically: all 46 are cited at least once and every
      citation resolves.
- [x] **Remove or qualify "network-bound".** Gone from the abstract and §1.
      §4.2 has a new "Client placement: loopback and co-located" paragraph;
      §5.2 discusses what co-location does to the worker-count result (changing
      vCPU count changes the client's CPU budget too, so the contrast varies two
      things at once); §7 Construct restates it.
- [x] **Explain the detectability floor in §7.** The Statistical paragraph now
      separates the two quantities: 0.6–0.75% is the *per-cell* floor for one
      strategy against `disabled` (0.58% naive, 0.69% clustered, ~0.75%
      simulated), whereas CARD − SYNC is a difference of two paired contrasts
      against a shared reference, aggregated over 120 cells, whose realised
      standard error over the eleven runs is 0.069 pp and whose own floor is
      therefore ≈ 0.19 pp — which is why 0.34 pp resolves.
- [x] **Use matched-load numbers in §5.2's extrapolation.** Both ends now come
      from the matched-load table: 0.790/0.175 = 4.51× at 4 workers,
      0.724/0.457 = 1.58× at 8, with an explicit note on why §5.1's 1.66× must
      not be paired with the 4-worker figure. The crossover is recomputed from
      the matched pair: ~11 workers (power law) to ~12 (linear), replacing the
      old "11–16". §1, §6 and §8 were updated to quote the ratios consistently.
- [x] **Resolve the "prior analysis" and "companion study" mentions.** Both
      deleted. §5.3 now states the fact directly (the comparison is the one
      this dataset does not settle, and it weakened from p ≈ 0.11 to 0.23 with
      the eleventh run) instead of attributing it to a prior analysis. Appendix
      A no longer names a companion study. See section 1 for the disposition of
      the draft itself.
- [x] **Fix the Appendix A paths and add the Zenodo DOI.** Paths verified from
      a fresh clone. A **Data Availability** section was added carrying the
      repository URL, the `v1.0-paper` tag and the DOI placeholder.

## 3. Paper: recommended

- [ ] **Run a small separate-client replication.** *Not done — requires new
      experiments, which were out of scope for this pass.* It is now written up
      in §7 as one of the three experiments that would address the stated
      limits, with the design specified (second `Standard_D8s_v6` in the same
      region, main contrasts at two or three concurrency levels, licensed by
      §5.2's concurrency-invariance result).
- [ ] **Test one widely used metrics library.** *Not done — same reason.*
      Specified in §7 and pointed to from §6: the same two contrasts built on
      the Rust `prometheus` or `metrics` crate, one labelled by command name and
      one by data key, to test whether the axis *ordering* survives a production
      library's fixed costs.
- [x] **Soften informal wording.** "Folklore" is gone (replaced by "practitioner
      guidance" / "received wisdom" / "the guidance we test"), as is "the honest
      present claim".
- [x] **Re-export all figures as vector PDFs.** All four figures now write a
      vector PDF alongside the PNG, with embedded Type 42 fonts (not Type 3,
      which several publishers reject) and text kept selectable. Figure widths
      were capped at 6.6 in and the two-panel figures restacked vertically so
      that a figure placed at the journal's text width is close to a 1:1
      reproduction and axis labels stay at or above 9 pt. PDF output is
      byte-deterministic.
- [x] **Trim keywords and drop the CCS Concepts block.** Keywords cut from nine
      to six; the CCS block removed.

## 4. Submission prep

- [ ] **Create an ORCID iD.** *Author action.* Placeholder in the manuscript and
      a commented-out line in `CITATION.cff`, both flagged.
- [x] **Set your affiliation.** "Independent Researcher, Delhi, India" on the
      manuscript, the cover letter and `CITATION.cff`.
- [ ] **Move the paper into the journal's LaTeX template.** *Author action —
      needs Wiley's `wileyNJD-v2` class from the journal's author-guidelines
      page.* The manuscript is venue-neutral Markdown with a regenerated Word
      export (`docs/final_paper_v28.docx`, rebuilt from the edited Markdown with
      all four figures embedded); SPE accepts an initial submission as a single
      PDF, so this can be deferred to revision. Use the vector PDFs in
      `docs/figures/`.
- [x] **Add a data-availability statement.** New **Data Availability** section
      in the manuscript with the repository URL, the release tag and the DOI
      placeholder.
- [x] **Prepare an AI-use declaration.** New **Declaration of Generative AI Use**
      section in the manuscript: states the uses (code, drafting, literature
      search) and states explicitly that no measurement was generated,
      simulated or altered by an AI tool.
- [x] **Write a one-page cover letter.** `submission/cover_letter.md`: the
      findings, the fit with SPE's aims and scope, the reproducibility story,
      and the declarations (original, not under review elsewhere, no competing
      interests, no funding, subscription option).
- [ ] **Choose the subscription option.** *Author action at submission time.*
- [ ] **Submit to one journal only.** *Author action.*
