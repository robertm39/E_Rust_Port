# Periodic ground-SAT trigger evaluation

This experiment evaluates Bead `E_Rust_Port-9jt.4.4`.

The implementation audit and frozen decision policy are in
`PREREGISTRATION.md`. `select_corpus.py` creates the candidate-blind
fresh-family `corpus.jsonl` from the tracked CASC-2025 manifest and prior SAT
study selections.

The ignored raw evidence belongs under
`.artifacts/experiments/2026-07-29-017-ground-sat-trigger-evaluation/`.
The completed findings are in `FINDINGS.md`; `RESULTS.md` is the compact
comparison, `results-summary.json` is the source analysis, and
`experiment-result.json` is the validated result contract.

The decision leaves periodic SATCheck default-off. The 5,000-step policy was
cheap but added no solve or common-solve benefit, the 10,000-step policy barely
fired, and the size-10,000 policy spent 55.8% of reached-run CPU in SATCheck
and regressed common-solve CPU to 1.543 times baseline.

The measured source revision was
`4e24b38c223617f7f2a55c23ab2295de7addd10e`. The complete held-out archive
has SHA-256
`b5f7da29bdbd5e6844d6188d9bf95ab8e4bef015bad116edc467f4e5be16ee5a`;
the proof/core validation archive has SHA-256
`2b4fae05e3d2fb57f4b3879abf71ebd906d9b47d7a14a2da218c04564832188f`.
