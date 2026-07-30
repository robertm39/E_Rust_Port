# Proof-lemma and watchlist transfer

This experiment evaluates Bead `E_Rust_Port-9jt.3.5`.

`PREREGISTRATION.md` freezes the source traces, family-held-out corpus,
same-category and cross-category transfer pools, explicit-lemma safety gate,
structure-matched treatments, budgets, metrics, and decision rule before any
candidate extraction or held-out search result is observed.

The experiment reuses the immutable corpus and successful training PCL traces
from experiment 018. Raw extracted traces, candidate pools, target-specific
wrappers, lemma-admissibility certificates, search outputs, telemetry, and
analysis belong under:

```text
.artifacts/experiments/2026-07-29-019-proof-lemma-watchlist-transfer/
```

The measured prover source revision is
`ce75ea3b68c34ab1640e0f362438a656626a5b0e`.

The final finding is negative for deployment: no explicit candidate passed the
target-axiom safety gate, and watchlists added no solve, hit, proof shortening,
or search-work reduction. See `FINDINGS.md`.

The complete ignored raw archive is
`.artifacts/experiments/2026-07-29-019-proof-lemma-watchlist-transfer/lemma-watchlist-019-complete.tar.gz`
(41,959,858 bytes, SHA-256
`fbae6d65079fb3677a89973f4453fff6044952102e865ae69ba167eb224b274a`).
