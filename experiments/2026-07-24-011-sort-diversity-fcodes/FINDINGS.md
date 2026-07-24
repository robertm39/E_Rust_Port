# Experiment 284: Sort diversity function codes

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the production diversity-weight evaluator count distinct function symbols
with one fresh vector plus `sort_unstable`/`dedup`, avoiding the per-symbol
`BTreeSet` nodes in `Clause::return_fcodes` while leaving that public
C-compatible ordered API unchanged?

## Setup

- Parent source: commit `361a7cdb`; executable source remains accepted
  Experiment 270.
- Fresh unchanged-source default-feature control: 8,991,960,325
  instructions.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The representative optimized line profile records 90,343 diversity
evaluations and 265,728,045 inclusive instructions in their
`Clause::return_fcodes` calls. Diversity consumes only the distinct-symbol
count; other callers may depend on the public function's first-encounter
output order.

## Candidate

The WFCB-only path retains fresh operation-local storage. It collects the same
unique term cells through `Clause::collect_subterms`, appends every nonvariable
function code to one vector, sorts and deduplicates that vector, and returns
its length. The public `Clause::return_fcodes`, term-operation flags, variable
scratch, and weight formula remain unchanged.

## Results

### Sort and deduplicate

Variant A executes 9,001,421,211 instructions. That is 9,460,886 or
0.105215% above the fresh unchanged-source control and 8,608,286 or
0.095724% above the archived accepted profile. Its hypothetical ratio to the
5,254,361,329-instruction C reference worsens to 1.713133.

Collecting every nonvariable occurrence before sorting performs more work than
the accepted `BTreeSet`, so this form fails the exact-instruction gate.

### First-seen linear vector

Variant B retains only the first occurrence of each function code in the fresh
vector through a linear scan. It preserves first-encounter order and improves
the exact profile to 8,943,762,201 instructions:

- 48,198,124 or 0.536014% below the fresh unchanged-source control;
- 49,050,724 or 0.545444% below the archived accepted profile;
- a hypothetical Rust/C ratio of 1.702160.

The matched default-feature native candidate is 8,933,888 bytes, 18,432 bytes
smaller than the 8,952,320-byte parent. Three parent and five candidate direct
proof runs are byte-identical and exit zero.

Native performance reverses the deterministic win. Two independent blocks
each exclude four alternating warmup pairs and retain 64 alternating measured
pairs:

| Block | Wall mean | CPU mean | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: |
| First 64 pairs | +1.031636% | +1.008484% | 26 | 27 | 5 |
| Second 64 pairs | +1.592559% | +1.534356% | 25 | 22 | 8 |
| Combined 128 pairs | +1.306800% | +1.266030% | 51 | 49 | 13 |

Positive percentages are candidate regressions. Combined wall and CPU medians
regress 1.465444% and 1.030928%; mean paired changes regress 1.482324% and
1.452494%.

The first block's last 32 pairs regress 0.786021% wall and 0.130293% CPU. The
second block is more decisive: its last 32 regress 1.658572% wall and
1.948270% CPU, and its last 16 regress 2.291082% wall and 1.812081% CPU.

## Validation

- All six focused diversity tests pass in default and all-feature modes for
  both variants.
- The focused regression compares the private count with the public ordered
  collector and repeats after variable operation flags are deliberately left
  stale.
- Corrected default-feature WSL Callgrind for both variants proves LUSK6 and
  exits zero.
- The parent and Variant B native fingerprints both record exactly
  `features=["default"]`.
- All direct, warmup, and 256 measured native processes prove and exit zero.
- The full compatibility matrix and repository-wide gates are skipped after
  the replicated native rejection.
- After rejection, the count-only helper, import, call site, and focused
  assertions are removed; accepted `diversityweight.rs` is restored
  byte-for-byte.

## Decision

Reject both variants. A first-seen linear vector removes a genuine 0.536014%
of default-feature instructions and shrinks the executable, but it regresses
native wall and CPU time in two independent blocks and stable tails. Preserve
the accepted `Clause::return_fcodes` path and Experiment 270 baseline at
8,992,812,925 instructions, or 1.711495 times C.

Function-code counting in the diversity evaluator is exhausted across fresh
`BTreeSet`, retained output, sort/dedup, and first-seen linear-vector shapes.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-011-sort-diversity-fcodes/rust-callgrind-linear-diversity-fcodes.out \
  target-wsl-284b-linear-diversity-fcodes/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-284-linear-diversity-fcodes\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-011-sort-diversity-fcodes\native-lusk.csv
```
