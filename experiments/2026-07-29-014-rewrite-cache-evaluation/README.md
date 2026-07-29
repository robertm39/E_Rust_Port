# Shared rewrite-link and normal-form cache evaluation

This experiment evaluates Bead `E_Rust_Port-9jt.7.4`.

Umlaut already stores canonical-term rewrite links and rule/full normal-form
dates. The study measures that implementation, adds stable search telemetry
for cache activity, and compares the normal build with a compile-time
proof-preserving cache ablation. The frozen protocol and decisions are in
[`PREREGISTRATION.md`](PREREGISTRATION.md), and the immutable CASC test
selection is in [`corpus.json`](corpus.json).

No benchmark result may be inspected before the preregistration commit.
Generated raw evidence belongs under
`.artifacts/experiments/2026-07-29-014-rewrite-cache-evaluation/`.

The study is complete. The existing full cache passed proof, polarity,
common-solve CPU, search-size, and larger-budget memory gates. Production
remains unchanged. See [`FINDINGS.md`](FINDINGS.md),
[`RESULTS.md`](RESULTS.md), and
[`results-summary.json`](results-summary.json).

`run.py` creates and resumes the two frozen execution contracts. `analyze.py`
checks those contracts and applies the preregistered decision. `verify.py`
selects one reproducible proof per solved category and build and runs the
integrity-pinned ProofCheck 1.0 path. `test_scripts.py` covers selection,
content-addressing, telemetry aggregation, paired ratios, and proof-category
selection.
