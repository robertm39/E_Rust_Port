# Experiment 245: Retain maximal literals in the candidate vector

## Question

Can maximal-literal marking retain surviving indices in the already allocated
candidate vector, eliminating the second result vector and its 90,906
production growth allocations while preserving C comparison order, flags, and
bank-backed error behavior?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `EqnList::mark_maximal_literals_with_bank` causes exactly 90,906
  `RawVec` growth calls in the accepted profile.
- Accepted whole-program Rust allocator calls: 4,380,910.

## Candidate

Keep surviving candidate indices as a processed prefix of the existing
candidate vector. A dominated candidate is removed at the prefix boundary; a
survivor advances the boundary. Later comparisons begin immediately after the
prefix, reproducing the former candidate sequence without repeatedly removing
survivors from index zero.

Maximal flags remain deferred until every comparison succeeds, preserving the
bank-backed function's partial-state behavior if ordering returns an error.
Both bank-backed and already prepared ordering paths use the same
single-vector representation.

## Validation

- The focused equation-list regression extends the existing C candidate-order
  test with a bank-backed list whose first literal is dominated and whose two
  remaining literals are equivalent maxima. All 21 equation-list tests pass.
- The full serial all-target/all-feature suite passes 4,388 library tests plus
  every integration and binary target.
- Focused proof report `.artifacts/e-compare/20260723-034848-974761` has GEO,
  HEN, LUSK6, and LUSK6ext exact, with zero mismatches at the standard
  60-second/2-GiB limits.
- Strict resource report `.artifacts/e-compare/20260723-035025-807612` has
  BOO020 and SWV851 exact, including their `ResourceOut` status and exit code
  at the standard 60-second/2-GiB limits.
- Maintained report `.artifacts/e-compare/20260723-035427-224719` completes all
  50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` normalized-output difference.
- Formatting, strict all-target/all-feature pedantic Clippy, the locked
  all-feature release build, all four C-source documentation gates,
  `git diff --check`, and vendored-C cleanliness pass.

## Measurement

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 9,898,434,766 instructions. This is 25,130,006 below the
9,923,564,772-instruction parent, a 0.253236% whole-prover reduction. The
Rust/C ratio improves from 1.888634 to 1.883851.

Exclusive instructions in `EqnList::mark_maximal_literals_with_bank` fall
from 12,545,028 to 10,090,566, a reduction of 2,454,462 or 19.565217%.
The coalesced `RawVec` growth symbol falls from 636,616 to 545,710 calls,
exactly the 90,906 growths attributed to the removed result vector. Total
Rust allocator calls fall from 4,380,910 to 4,290,002, a reduction of 90,908
or 2.075094%.

Native validation used two four-pair warmups followed by three independently
started 64-pair alternating blocks. All 384 measured processes prove and exit
zero. Across the combined 192 pairs, candidate-versus-parent wall mean is
effectively tied at +0.015742%, while process-CPU mean improves 0.056956%.
Wall median improves 0.011427%; the quantized CPU median regresses 0.917431%.
Mean paired changes are +0.172089% wall and +0.104952% CPU. The candidate wins
95 wall pairs and 89 CPU pairs, with 16 CPU ties. Individual block directions
alternate, so the native result is neutral rather than evidence of either a
regression or an additional gain. Both executables are exactly 8,654,336
bytes.

## Result

Accept. The single-vector state machine reproduces the former candidate
sequence and comparison order, retains stable literal storage, and continues
to defer all maximal-flag writes until bank-backed ordering has succeeded. It
removes every growth allocation owned by the redundant result vector,
improves deterministic instructions, is neutral across a large native sample,
and passes the complete proof, resource, and repository-wide acceptance
gates. The accepted baseline becomes 9,898,434,766 instructions, or 1.883851
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-single-maximal-vector.out \
  target-wsl-245-single-maximal-candidate-vector/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-245-single-maximal-candidate-vector\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-007-single-maximal-candidate-vector\native-lusk.csv
```
