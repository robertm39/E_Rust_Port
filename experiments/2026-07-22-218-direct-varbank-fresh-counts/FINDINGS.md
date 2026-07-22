# Rejected direct VarBank fresh counters

## Question

Can `VarBank` replace its ordered map of per-type fresh-variable cursors with
a dense vector indexed by the shared `TypeBank` UID, matching the upstream C
sort-stack representation and avoiding a tree lookup in `get_fresh_var`?

## Setup

- Parent source: commit `2489f4b6` (`Record rejected indexed term comparison`),
  whose executable source remains accepted Experiment 214.
- Candidate: change only `v_counts` from
  `BTreeMap<TypeUniqueId, usize>` to an automatically growing `Vec<usize>`.
  Keep the sparse variable table and the per-type `varstacks` map unchanged.
  The candidate also reuses the single `varstacks.entry` lookup on the
  existing-variable path.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-218-direct-varbank-fresh-counts/rust-callgrind-direct-varbank-fresh-counts.out`.
- Native timing: default-feature release binaries, two unrecorded alternating
  warmup pairs, then 64 alternating measured pairs. `native-lusk.csv` is the
  primary warmed sample. `native-lusk-initial.csv` retains the first sample,
  whose first 12 pairs were visibly distorted by host warmup/load.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate reaches the expected LUSK6 proof and falls from 10,632,641,385
to 10,592,796,772 instructions, a reduction of 39,844,613 or 0.374739%. The
hypothetical C/Rust ratio improves from 2.023584 to 2.016001.

`VarBank::get_fresh_var` falls from 54,637,167 to 34,806,054 exclusive
instructions, saving 19,831,113 or 36.296013% locally and explaining 49.77%
of the whole-program instruction reduction. The PD-tree cursor and
substitution normalization reproduce exactly at 1,697,827,541 and
437,245,456 instructions. `TermTree::insert` rises slightly by 212,312
instructions, from 658,651,917 to 658,864,229, consistent with ordinary
whole-binary layout movement.

## Native result

The warmed production-binary gate decisively reverses the Callgrind result.
Across all 64 alternating pairs:

- wall-time means are 2.152870142 seconds for the parent and 2.201299467 for
  the candidate, a 2.249524% aggregate regression;
- wall medians regress 1.529732%, and paired wall deltas regress 2.365973% by
  mean and 1.401149% by median;
- process-CPU means are 2.097656250 seconds for the parent and 2.146240234 for
  the candidate, a 2.316108% aggregate regression;
- CPU medians regress 2.290076%, and paired CPU deltas regress 2.432169% by
  mean and 1.148705% by median;
- the candidate wins only 23 of 64 wall pairs and 19 CPU pairs, with two CPU
  ties.

The conclusion is robust to trimming the two largest candidate wall samples:
trimmed wall mean and median still regress 1.507672% and 1.504755%; trimmed
CPU mean and median regress 1.613097% and 2.290076%. The final 32 pairs also
regress in wall and CPU mean and median. All 128 measured runs prove and exit
zero. The candidate executable is 8,643,584 bytes, 1,536 bytes smaller than
the 8,645,120-byte parent, so file size does not explain acceptance.

## Validation

- All 18 candidate VarBank tests pass, including dense-counter growth beyond
  the initial sort-stack capacity.
- All nine focused substitution tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The release candidate reaches the expected unsatisfiable result and exits
  zero under Callgrind.
- After rejection, the source was restored byte-for-byte; the accepted source
  has 17 VarBank tests and all nine substitution tests passing.
- Compatibility matrices were skipped because the production native gate
  rejected the performance-only representation change.

## Decision

Reject and restore the ordered `v_counts` map. Dynamic instruction count is
not a sufficient proxy for this representation: the direct vector removes
the intended lookup work under instrumentation but reliably slows the actual
production binary. Do not extend this candidate to `varstacks` without a new
layout hypothesis and a native-first measurement. Keep the accepted baseline
at 10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```powershell
.\run-native.ps1 `
  -ParentExe ..\..\target\native-214-move-termtree-insert-links\release\eprover.exe `
  -CandidateExe ..\..\target\native-218-direct-varbank-fresh-counts\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-varbank-fresh-counts.out \
  target-wsl-218-direct-varbank-fresh-counts/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
