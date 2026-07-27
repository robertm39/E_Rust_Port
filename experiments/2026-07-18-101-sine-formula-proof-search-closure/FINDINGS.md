# SInE Formula-owner Proof-search Closure

## Status

Completed for Bead `E_Rust_Port-j76.2.37`. The migrated gap was implemented by
the later formula-owner and destructive SInE integration slices. Fresh
threshold and GSinE proof-search comparisons close the wording-specific owner,
phase-order, and accounting surface. The vendored C checkout remained
unchanged.

## Ownership and phase decision

Unchanged C documents `ProofStateSinE` as destructive. It counts both live
clause and formula axiom sets, adds both owners to a temporary `StructFOFSpec`,
selects the requested objects, allocates fresh proof-state clause/formula sets,
moves the selected objects into those owners, and returns the combined
cardinality delta. The main executable adds that delta to relevance removals
before calling `FormulaSetCNF2`.

Rust keeps the same observable transition with safe owners. Threshold selection
uses the combined `ProofState::axiom_count` and clears both owner sets when the
threshold is exceeded. GSinE builds its relation over both live sets, selects
stable clause identifiers and allocation-unique formula entry ids, drains the
selected objects into fresh sets, replaces both proof-state owners, and returns
the combined delta. The ordinary proof-search driver adds that value to
relevance removals before formula CNF.

[`audit_sine_owners.py`](audit_sine_owners.py) passes all 16 C-source,
Rust-source, phase-order, accounting, permanent-regression, and fresh-reference
checks.

## Exact proof-search projections

[`compare_proof_search.py`](compare_proof_search.py) runs isolated unchanged C
at commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release on two
FOF fixtures. Both retained projections match exactly:

- `Threshold(1)` parses two formula axioms, removes both, generates zero initial
  clauses, closes under the restricted calculus with `GaveUp`, exits `10`, and
  writes no stderr.
- `GSinE(CountTerms,,false,10.0,,2,10,1.0)` parses four formula owners, removes
  only `irrelevant`, documents `goal`, `link`, and `far` before proof-state
  initialization, starts saturation with three clauses, finds a theorem, exits
  `0`, and writes no stderr.
- The selected strategy, eight owner/search statistics, completion line, SZS
  status, exit code, and stderr match in both cases.

The full GSinE stdout still contains already-tracked proof-document presentation
differences: platform-specific source paths and additional Rust canonization
documentation. Those do not change selected owners, phase order, clauses
entering saturation, proof result, or this issue's combined removal accounting;
the broader output surface remains owned by the proof-output and main-executable
comparison beads.

The retained [`reference.json`](reference.json) has SHA-256
`F2A7EE2DD99946257C03264A21E163B9CF35480A1A9F5FECFF2A4DC92A2336D4`.
The retained [`owner-audit.json`](owner-audit.json) has SHA-256
`111782ECC4F6BA97FB19D827ADF239DDE7241C4CEAB5BB367500764A27D88276`.
The compared Rust `eprover.exe` has SHA-256
`E4CAB1204C7F57AA50BDA1CD71FE869FFB4FC466A01506311A95C51E7F488A69`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-101-sine-formula-proof-search-closure\audit_sine_owners.py `
  --reference experiments\2026-07-18-101-sine-formula-proof-search-closure\reference.json `
  --output target\sine-formula-owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-101-sine-formula-proof-search-closure\compare_proof_search.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\sine-formula-proof-search.json `
  --expected experiments\2026-07-18-101-sine-formula-proof-search-closure\reference.json
```

## Compatibility decision

No additional owner or preprocessing API is needed for this migrated item.
Represented FOF formula owners participate destructively in threshold and GSinE
selection before CNF, and their combined removed count reaches proof-search
statistics exactly like C.
