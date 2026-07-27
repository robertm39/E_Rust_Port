# Rejected borrowed term-type comparison

## Question

Can structural term ordering borrow the two optional `Type` handles stored in
term links instead of cloning their `Rc` handles solely for `TypesCmp`?

## Setup

- Parent source: commit `bd1a18f7` (`Record rejected direct VarBank
  counters`), whose executable source remains accepted Experiment 214.
- Candidate: add a crate-private `Term::borrow_type` view and hold two shared
  `Ref` guards inside `compare_term_types`. Type presence, `TypesCmp`, pointer
  identity, structural ordering, and every public owning accessor are
  unchanged.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-219-borrowed-term-type-compare/rust-callgrind-borrowed-term-type-compare.out`.
- Native timing: default-feature release binaries, two alternating warmup
  pairs, then two independent blocks of 64 alternating measured pairs.
  `native-lusk-a.csv` and `native-lusk-b.csv` retain all 256 measured runs.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate reaches the expected LUSK6 proof and falls from 10,632,641,385
to 10,611,008,030 instructions, a reduction of 21,633,355 or 0.203462%. The
hypothetical C/Rust ratio improves from 2.023584 to 2.019467.

The gain is completely localized. In the parent, the two visible structural
comparison symbols plus `compare_term_types` total 284,017,905 exclusive
instructions. In the candidate, the structural comparison symbols plus the
now-standalone `types_cmp` total 262,384,106, saving 21,633,799 or 7.617055%
and explaining 100.002% of the whole-program reduction. The PD-tree cursor
and substitution normalization reproduce exactly at 1,697,827,541 and
437,245,456 instructions.

## Native result

Production timing rejects the candidate in both independent blocks despite
the stable instrumented win.

Block A has a strong time-localized reversal between its halves, so no single
half is representative. Across all 64 pairs, candidate wall mean and median
regress 1.385687% and 0.310471%; paired wall mean and median regress 1.557453%
and 0.100451%. CPU mean and median regress 1.985900% and 0.636943%; paired CPU
mean and median regress 2.170815% and 0.324675%. The candidate wins 32 wall
pairs and 28 CPU pairs, with four CPU ties.

The independent Block B repeats the direction: wall mean regresses 0.554223%,
paired wall mean and median regress 0.997192% and 0.353088%, CPU mean regresses
0.861152%, and paired CPU mean and median regress 1.215753% and 0.749074%.
Its aggregate wall median improves 0.593930%, but the candidate wins only 27
wall pairs and 24 CPU pairs, with three CPU ties.

Across all 128 pairs:

- wall mean and median regress 0.985048% and 0.615732%;
- paired wall mean and median regress 1.277323% and 0.310091%;
- CPU mean and median regress 1.442727% and 1.307190%;
- paired CPU mean and median regress 1.693284% and 0.746269%;
- the candidate wins 59 of 128 wall pairs and 52 CPU pairs, with seven CPU
  ties.

All 256 measured runs and all four warmup runs prove and exit zero. The
candidate executable is 8,632,320 bytes, 12,800 bytes smaller than the
8,645,120-byte parent, so neither dynamic instruction count nor file size
predicts its production timing on this host.

## Validation

- All 46 candidate term-function tests pass, including the structural
  comparison regression with distinct variable types.
- All 18 term-cell tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The release candidate reaches the expected unsatisfiable result and exits
  zero under Callgrind.
- After rejection, source is restored byte-for-byte; the same 46 and 18
  focused tests and formatting pass.
- Compatibility matrices were skipped because the production native gate
  rejected this performance-only ownership change.

## Decision

Reject and restore owning `Term::type_` calls in `compare_term_types`. The
borrowed guards eliminate reference-count work under Callgrind but slow both
independent production timing blocks. Keep the accepted baseline at
10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```powershell
.\run-native.ps1 `
  -ParentExe ..\..\target\native-214-move-termtree-insert-links\release\eprover.exe `
  -CandidateExe ..\..\target\native-219-borrowed-term-type-compare\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-borrowed-term-type-compare.out \
  target-wsl-219-borrowed-term-type-compare/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
