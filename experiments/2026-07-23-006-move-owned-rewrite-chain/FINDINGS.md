# Experiment 244: Move owned rewrite-chain handles

## Question

Can the recursive plain normal-form loop move its already-owned `Term` handle
into top-level rewrite-chain traversal, eliminating a redundant `Rc` clone and
identity comparison on a roughly 3.9-million-call hot path?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `term_follow_top_rw_chain`: 118,115,006 exclusive instructions.
- Accepted Rust allocator calls: 4,380,910.

## Candidate

Keep the existing borrowed rewrite-chain API for general callers, but route it
through an internal owned helper. The recursive normal-form loop saves the
input pointer identity, moves its owned handle into that helper, and compares
the returned pointer identity instead of retaining a cloned handle for the
old/new identity comparison.

This changes neither rewrite eligibility nor link traversal. A focused
regression covers both traversed and restricted, non-traversed owned paths.

An initial encoding returned a third `traversed` Boolean from the helper. It
was abandoned after Callgrind rose to 9,939,213,353 instructions
(+15,648,581, +0.15769%): helper bookkeeping added 4.15 million instructions,
while the normal-form owner added about 9.46 million. The pointer-identity
refinement retains the baseline two-result traversal loop.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --lib --all-features -- -D warnings -D clippy::pedantic`
- The focused owned-traversal regression passed.
- All three `follow_top_rw_chain` tests passed, covering unrestricted,
  restricted, `SoS`, and non-demodulator termination behavior.
- Both deterministic candidates found the exact LUSK6 proof, reported
  `Unsatisfiable`, and exited zero.
- Direct native parent/candidate runs had byte-exact stdout and stderr,
  including `% Proof found!` and `% SZS status Unsatisfiable`.
- All 128 measured native runs exited zero.

## Measurement

The refined pointer-identity candidate falls from 9,923,564,772 to
9,922,066,170 Callgrind instructions: a reduction of 1,498,602 or 0.015101%.
The implied C/Rust ratio moves from 1.888634 to 1.888349.

The owned helper falls from 118,115,006 to 110,262,172 instructions, down
7,852,834 or 6.648464%. Both normal-form owners regress, however:

- `term_li_normalform_plain_with_date'2`: 276,321,216 to 282,160,378,
  up 5,839,162 or 2.113179%.
- `term_li_normalform_plain_with_date`: 76,593,627 to 76,930,003,
  up 336,376 or 0.439170%.

The intended helper and its two callers therefore improve by only 1,677,296
instructions together, while work outside them rises by 178,694.

Four alternating warmup pairs preceded 64 alternating measured native pairs.
Positive percentages mean the candidate is faster. Every aggregate is
negative, and the stable last half is no better:

| Native metric | All 64 pairs | Last 32 pairs |
| --- | ---: | ---: |
| Wall mean | -2.113475% | -1.962238% |
| CPU mean | -1.665675% | -1.739927% |
| Wall median | -1.086741% | -1.560024% |
| CPU median | -1.923077% | -0.975610% |
| Mean paired wall change | -2.193572% | -2.006174% |
| Mean paired CPU change | -1.761703% | -1.755694% |
| Median paired wall change | -1.750851% | -1.674302% |
| Median paired CPU change | -1.405951% | -1.000100% |
| Candidate wall wins | 18 of 64 | 7 of 32 |
| Candidate CPU wins | 21 of 64, 3 ties | 8 of 32, 3 ties |

The native candidate and accepted binaries are both 8,654,336 bytes.

## Result

Reject. Moving the owned handle can remove measurable reference-count work,
but the surrounding owner code becomes more expensive and the tiny net
Callgrind improvement does not translate to native execution. The candidate
is decisively slower in both the full sample and stable half.

Compatibility matrices and full repository gates were skipped after the
native rejection. All candidate production code and tests were removed;
accepted Experiment 231 is restored byte-for-byte at 9,923,564,772
instructions and 1.888634 times C.

Raw evidence:

- Initial Boolean candidate:
  `.artifacts/experiments/2026-07-23-006-move-owned-rewrite-chain/rust-callgrind-move-owned-rewrite-chain.out`
- Refined pointer-identity candidate:
  `.artifacts/experiments/2026-07-23-006-move-owned-rewrite-chain/rust-callgrind-move-owned-rewrite-chain-identity.out`
- Warmups:
  `.artifacts/experiments/2026-07-23-006-move-owned-rewrite-chain/native-warmup.csv`
- Measured pairs:
  `experiments/2026-07-23-006-move-owned-rewrite-chain/native-lusk.csv`

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-move-owned-rewrite-chain-identity.out \
  target-wsl-244-move-owned-rewrite-chain/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-244-move-owned-rewrite-chain\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-006-move-owned-rewrite-chain\native-lusk.csv
```
