# Experiment 238: First-order substitution dereference

## Question

Can `Substitution::norm_term` match the maintained non-LFHO C reference by
selecting a free-variable-only dereference path once per first-order
normalization call, while retaining the existing applied-variable-capable path
for higher-order problems?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `Substitution::norm_term`: 437,245,456 exclusive instructions.
- Experiment 237 attributes 67,944,731 instructions inside the normalizer to
  the applied-free-variable test and its callees on 3,260,660 nonvariable
  visits.

## Candidate

- Dispatch once at `Substitution::norm_term` entry on the thread-local problem
  type.
- Monomorphize the existing traversal as separate first-order and higher-order
  const-generic bodies, so there is no per-node mode branch.
- For first-order calls, use a specialized always-dereference helper that
  follows only ordinary free-variable bindings and preserves the existing
  two-link borrowing optimization.
- For higher-order calls, retain the existing applied-free-variable-capable
  `term_deref_always`.
- Add focused coverage for long first-order binding chains and for the
  intentional non-expansion of applied free variables on the first-order
  helper.

The maintained C performance reference is configured without `ENABLE_LFHO`;
its `TermDerefAlways` therefore follows only `term->binding`. The separately
archived higher-order C reference enables the applied-variable branch at
compile time. Rust supports both modes in one executable, so the candidate
uses runtime selection at the outer normalization boundary.

## Validation

- All 19 focused term-cell tests and all 9 focused substitution tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The deterministic candidate reaches the exact LUSK6 proof, reports
  `Unsatisfiable`, processes the expected 4,873 clauses, and exits zero.
- Direct native parent/candidate runs have byte-exact stdout and stderr,
  including `% Proof found!` and `% SZS status Unsatisfiable`.
- All 128 measured native runs exit zero.

The candidate Callgrind profile retires 9,903,837,275 instructions, down
19,727,497 or 0.198794% from the 9,923,564,772-instruction baseline. The
implied C/Rust ratio improves from 1.888634 to 1.884879.

Four alternating warmup pairs preceded 64 alternating measured native pairs.
Positive percentages mean the candidate is faster; every measured aggregate
is negative:

| Native metric | All 64 pairs | Last 32 pairs |
| --- | ---: | ---: |
| Wall mean | -1.269467% | -3.040038% |
| CPU mean | -1.631013% | -3.011472% |
| Wall median | -2.225740% | -4.974493% |
| CPU median | -2.692308% | -3.488372% |
| Mean paired wall change | -1.464218% | -3.229169% |
| Mean paired CPU change | -1.824480% | -3.157975% |
| Median paired wall change | -0.274677% | -2.394107% |
| Median paired CPU change | -0.772212% | -2.687027% |
| Candidate wall wins | 32 of 64 | 13 of 32 |
| Candidate CPU wins | 28 of 64, 2 ties | 12 of 32 |

The candidate binary also grows 3,072 bytes, from 8,654,336 to 8,657,408
bytes. The instruction reduction therefore does not translate to native
throughput. The measurements do not isolate whether the dominant cause is the
new thread-local problem-type read, duplicated optimized traversal layout, or
their interaction.

## Result

Reject. The deterministic instruction reduction is real and the intended
first-order semantics are preserved, but the replicated native regression is
substantially larger and worsens in the stable half. Compatibility matrices
and full repository gates were skipped after the candidate failed its native
performance requirement.

All candidate production code and tests were removed. The accepted source is
restored byte-for-byte to Experiment 231 at 9,923,564,772 instructions and
1.888634 times C.

Raw evidence:

- Callgrind:
  `.artifacts/experiments/2026-07-22-238-first-order-subst-deref/rust-callgrind-first-order-subst-deref.out`
- Warmups:
  `.artifacts/experiments/2026-07-22-238-first-order-subst-deref/native-warmup.csv`
- Measured pairs:
  `experiments/2026-07-22-238-first-order-subst-deref/native-lusk.csv`

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-first-order-subst-deref.out \
  target-wsl-238-first-order-subst-deref/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-238-first-order-subst-deref\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-22-238-first-order-subst-deref\native-lusk.csv
```
