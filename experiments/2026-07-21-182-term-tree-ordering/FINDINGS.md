# Direct term-tree ordering results

## Question

Can the term splay tree consume `Ordering` directly instead of asking the hot
top-cell comparator for an integer and immediately comparing that result with
zero at every find, insert, extract, and rotation decision?

## Setup

- Parent source: commit `1704afa3` (`Specialize first-order PD-tree
  expansion`); commit `f2f1e83c` adds only rejected-experiment evidence.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-180-pdt-first-order-expansion/rust-callgrind-fo-expansion.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-182-term-tree-ordering/rust-callgrind-ordering.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

C `TermTopCompare` returns a signed `long`, but every term-tree caller consumes
only whether the result is less than, equal to, or greater than zero. Rust
previously reproduced that integer result and then converted it to `Ordering`
at every tree branch.

The private comparator now returns `Ordering` directly from function-code,
higher-order type-identity, arity, and argument-identity comparisons. Find,
insert, extract, and the top-down splay loop consume that value without a
second comparison. The public Rust comparator remains an `i64` API and maps
the private result to `-1`, `0`, or `1`, preserving the C-documented sign
contract. Key order, first-order debug assertions, higher-order type ordering,
pointer identity, and intrusive tree links are unchanged.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,798,391,251
instructions. This is 37,689,467 below the 11,836,080,718-instruction parent,
a 0.3184% whole-prover reduction. The deterministic C/Rust ratio improves from
2.2526 to 2.2454.

The intended term-tree boundary accounts for the reduction:

| Metric | Parent | Candidate | Change |
| --- | ---: | ---: | ---: |
| Whole prover | 11,836,080,718 | 11,798,391,251 | -37,689,467 (-0.3184%) |
| Top-cell comparator exclusive | 528,924,305 | 512,581,477 | -16,342,828 (-3.0898%) |
| `splay_term_tree` exclusive | 218,240,785 | 204,236,401 | -14,004,384 (-6.4169%) |
| `TermTree::insert` exclusive | 133,867,478 | 126,526,904 | -7,340,574 (-5.4835%) |
| C/Rust ratio | 2.2526 | 2.2454 | -0.0072 |

Dominant PD-tree, rewrite, dereference, evaluation-index, allocator, and term-
bank entries remain unchanged in the compact exclusive profile.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-150855-753510/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-151054-173545/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-151507-886436/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- All four focused term-tree tests pass.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept the private `Ordering` comparator path. It preserves the complete C key
and public sign contract, removes a redundant result conversion at every hot
tree decision, localizes a 0.3184% whole-prover reduction to comparator and
splay work, and passes the complete proof/resource matrix. Keep the main
performance issue open: the deterministic ratio remains 2.2454 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-ordering.out \
  target-wsl-182-term-tree-ordering/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-182-term-tree-ordering
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-182-term-tree-ordering\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-182-term-tree-ordering\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
