# Experiment 243: Accumulate top-insert properties

## Question

Can fresh term-bank metadata accumulate direct and child-derived flags in one
local `TermProperties` value, replacing repeated `Cell` read/modify/write
operations with one final property write while preserving every propagated
flag and count?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `TermBank::term_top_insert`: 260,571,383 exclusive instructions.
- Its source attribution includes 37,572,080 instructions in
  `Cell`/`RefCell` operations.

## Candidate

Read the fresh term's existing properties once, accumulate direct flags and
child-derived flags in a local `TermProperties`, and write the final value
once after counts and weight are computed. Seven unconditionally inherited
child flags are read through one combined mask; eta-expandability remains a
separate conditional read because lambda/application heads suppress it.

No callback or shared owner can observe intermediate property states: this
helper runs only for a newly inserted, not-yet-returned top cell. Applied-free-
variable normalization reads shape and arguments rather than properties.

A regression covers preservation of an unrelated existing flag, every
inherited property, Boolean child typing, variable/function counts, weight,
and non-ground status.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --lib --all-features -- -D warnings -D clippy::pedantic`
- All 123 `terms::termbanks::tests` passed, including the new complete
  property-propagation regression and existing ground, applied-variable,
  higher-order, duplicate-property, parser, and garbage-collection cases.
- The deterministic candidate found the exact LUSK6 proof, reported
  `Unsatisfiable`, and exited zero.
- Direct native parent/candidate runs had byte-exact stdout and stderr,
  including `% Proof found!` and `% SZS status Unsatisfiable`.
- All 128 measured native runs exited zero.

## Measurement

Callgrind falls from 9,923,564,772 to 9,909,915,514 instructions: a reduction
of 13,649,258 or 0.137544%. The implied C/Rust ratio improves from 1.888634 to
1.886036.

The intended `TermBank::term_top_insert` owner falls from 260,571,383 to
248,381,135 instructions, down 12,190,248 or 4.678276%.
`TermCellStore::insert` remains exactly 127,990,740 instructions.
`TermTree::insert` rises only 221,050 instructions, while work outside these
three owners improves by 1,680,060.

Four alternating warmup pairs preceded 64 alternating measured native pairs.
Positive percentages mean the candidate is faster. The full sample is
negative, and the stable half remains wall-negative:

| Native metric | All 64 pairs | Last 32 pairs |
| --- | ---: | ---: |
| Wall mean | -0.585077% | -0.367602% |
| CPU mean | -0.891862% | -0.032000% |
| Wall median | -0.772300% | -0.334659% |
| CPU median | -1.030928% | -0.515464% |
| Mean paired wall change | -0.514792% | -0.301971% |
| Mean paired CPU change | -0.807795% | +0.055482% |
| Median paired wall change | -0.789592% | -0.666871% |
| Median paired CPU change | -1.041667% | 0.000000% |
| Candidate wall wins | 22 of 64 | 13 of 32 |
| Candidate CPU wins | 21 of 64, 5 ties | 14 of 32, 3 ties |

The candidate binary shrinks 1,024 bytes, from 8,654,336 to 8,653,312 bytes.

## Result

Reject. Local property accumulation removes real deterministic work and
preserves all tested behavior, but the native executable is slower across the
full sample and remains wall-negative in the stable half. The instruction and
size reductions therefore do not satisfy the port's native performance
requirement.

Compatibility matrices and full repository gates were skipped after the
native rejection. All candidate production code and tests were removed;
accepted Experiment 231 is restored byte-for-byte at 9,923,564,772
instructions and 1.888634 times C.

Raw evidence:

- Callgrind:
  `.artifacts/experiments/2026-07-23-005-accumulate-top-insert-properties/rust-callgrind-accumulate-top-insert-properties.out`
- Warmups:
  `.artifacts/experiments/2026-07-23-005-accumulate-top-insert-properties/native-warmup.csv`
- Measured pairs:
  `experiments/2026-07-23-005-accumulate-top-insert-properties/native-lusk.csv`

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-accumulate-top-insert-properties.out \
  target-wsl-243-accumulate-top-insert-properties/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-243-accumulate-top-insert-properties\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-005-accumulate-top-insert-properties\native-lusk.csv
```
