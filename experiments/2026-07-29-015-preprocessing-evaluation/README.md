# Blocked-clause and definition-oriented preprocessing evaluation

This experiment evaluates Bead `E_Rust_Port-9jt.7.1`.

The static inventory is in `transformation-inventory.json`. It shows that
Umlaut already has production implementations of the relevant E-style
definition and clause transformations. The measured candidates are the three
implemented but generated-schedule-inactive options: blocked-clause
elimination, singular predicate elimination, and TWEE-style goal definitions.

The frozen protocol and decisions are in `PREREGISTRATION.md`. Generated raw
evidence belongs under
`.artifacts/experiments/2026-07-29-015-preprocessing-evaluation/`.

The completed findings are in `FINDINGS.md`; `RESULTS.md` is the compact table
and `results-summary.json` is the machine-readable report. The final decision
keeps all three candidates default-off: each preserved correctness and had
measurable held-out reach, but none added a solve or met the preregistered CPU
benefit threshold.

The measured source revision was `23a8a9700dffb18df57502cb600accaee3513887`.
The complete ignored evidence archive has SHA-256
`6da8483fe84a98e328e1d08e8a457451a962b1ca338204934cddf2389ef4f733`.
