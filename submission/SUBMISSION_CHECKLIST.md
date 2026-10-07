# Submission checklist

Target: **Software: Practice and Experience** (Wiley). Backup: **Journal of
Systems and Software** (Elsevier). Submit to **one** journal at a time, through
the publisher's own website.

Status key: `[x]` done in this repository · `[ ]` requires an action only the
author can take (an account, a payment choice, a website form).

## Done in the repository

- [x] Abstract cut to 250 words; run counts, intervals and p-values moved into
      the body.
- [x] Unsupported claims cited: the OpenTelemetry/instrumentation overhead
      figures and the cardinality-as-backend-cost framing in §2, and the
      "keep the hot path lock-free" guidance in §1.
- [x] HdrHistogram cited in the text where the collector is introduced (§3.2).
- [x] Bibliography grown from 20 to 46 references, including Tokio, DashMap,
      the Rust HdrHistogram port, the Rust `prometheus` and `metrics` crates,
      recent instrumentation-overhead studies and time-series/cardinality work.
      Every reference is cited at least once; every citation resolves.
- [x] "Network-bound" removed from the abstract and §1; §4.2 describes the
      setup as a loopback, co-located client, and §5.2 and §7 discuss what that
      does to the worker-count result.
- [x] §7 states which quantity the 0.6–0.75% detectability floor applies to
      (a single cell) and why the 0.34 pp headline is resolved anyway
      (its own floor is 0.19 pp).
- [x] §5.2's extrapolation uses matched-load numbers throughout
      (4.51× at 4 workers, 1.58× at 8) and the crossover is recomputed from
      them (~11–12 workers, was "11–16").
- [x] "Prior analysis" and "companion study" mentions removed. The earlier
      draft is in `archive/paper_draft.md`, marked explicitly as superseded,
      never submitted anywhere, and not cited by the manuscript. There is
      nothing to disclose to the editor as a related submission.
- [x] Appendix A paths match the repository exactly, and a data-availability
      statement has been added.
- [x] Informal wording softened ("folklore", "the honest present claim").
- [x] All four figures re-exported as vector PDFs alongside the PNGs, with
      embedded Type 42 fonts and type that stays at or above 9 pt when a
      figure is placed at the journal's text width.
- [x] Keywords trimmed to six; the ACM CCS Concepts block removed.
- [x] Affiliation set to "Independent Researcher, Delhi, India".
- [x] AI-use declaration added to the manuscript.
- [x] Cover letter drafted (`submission/cover_letter.md`).
- [x] `CITATION.cff` added so the artifact can be cited correctly.
- [x] Reproduction verified from a fresh clone (see
      `submission/reproduction_log.md`).

## Author actions still required

- [ ] **Create an ORCID iD** at <https://orcid.org> (free). Then replace the
      `0000-0000-0000-0000` placeholder in `docs/final_paper_v27.md` and
      uncomment the `orcid:` line in `CITATION.cff`.
- [ ] **Tag and publish the GitHub release.** The tag `v1.0-paper` is created
      locally; push it (`git push origin v1.0-paper`) and publish a release
      from it on GitHub.
- [ ] **Archive the release on Zenodo** to mint a DOI. Enable the repository in
      Zenodo's GitHub integration *before* publishing the release, or upload
      the release tarball manually. Then replace `10.5281/zenodo.XXXXXXX` in:
      - `docs/final_paper_v27.md` (Data Availability section)
      - `CITATION.cff` (uncomment the `doi:` line)
      - `README.md`
- [ ] **Move the manuscript into the journal's LaTeX template.** For SPE that
      is Wiley's `wileyNJD-v2` class, downloaded from the journal's author
      guidelines page; for JSS it is Elsevier's `elsarticle`. The manuscript is
      currently venue-neutral Markdown (`docs/final_paper_v27.md`) with a Word
      export (`docs/final_paper_v27.docx`); SPE accepts an initial submission
      as a single PDF, so the LaTeX conversion can be deferred to revision if
      preferred. Use the vector PDFs in `docs/figures/` for the figures.
- [ ] **Choose the subscription (non-open-access) option** when the submission
      system asks, so no article-processing charge is incurred.
- [ ] **Submit through the publisher's official site only**, and to one journal
      at a time.
