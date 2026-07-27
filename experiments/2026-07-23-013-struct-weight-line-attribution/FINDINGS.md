# Experiment 251: Structural-weight comparison line attribution

## Status

Completed as a diagnostic experiment for Bead `E_Rust_Port-j76.5.3`.

## Question

Attribute the accepted LUSK6 cost of `term_struct_weight_compare` to optimized
source lines before changing its C-shaped recursive comparison. The audit will
separate:

- cached weight and property access;
- free/de-Bruijn-variable classification;
- arity checks;
- immutable argument borrowing;
- nullable argument-slot checks and recursive descent.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851

## Method

Build the unchanged source with optimized release code plus line tables:

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 \
  CARGO_TARGET_DIR=target-wsl-251-struct-weight-lines \
  cargo build --locked --release --bin eprover
```

Then run the exact accepted LUSK6 Callgrind command and annotate
`src/terms/termfunc.rs`.

The profiled executable proved LUSK6 and retired 9,897,268,886 instructions.
That is 1,165,880 instructions, or 0.011778%, below the accepted baseline, so
the line-table build is representative. Its Rust/C ratio is 1.883629.

## Findings

The recursive structural comparator is algorithmically equivalent to
`TermStructWeightCompare` in `eprover/TERMS/cte_termfunc.c`: compare the cached
standard weights, distinguish variables, compare arities, and then recurse
lexicographically through argument slots. Rust's largest local overhead is
access through `RefCell`-backed argument vectors rather than a missing C
decision or shortcut.

Source attribution establishes these dynamic counts:

| Operation | Calls or visits | Attributed instructions |
| --- | ---: | ---: |
| Top-level comparator calls | 6,093,314 | 18,279,942 at function entry |
| Arity accessors | 711,744 | 5,693,952 in callees |
| Comparisons reaching the arity check | 355,872 | 9,483,678 at the call line |
| Argument borrows per side | 350,229 | 14,628,258 in callees |
| Comparisons reaching argument traversal | 350,229 | 10,945,886 at borrow lines |
| Recursive child comparisons | 508,375 | 315,984,782 inclusive |

Only about 5,643 comparisons have unequal arity: 355,872 comparisons reach the
arity check, while 350,229 continue to argument traversal. The current code
therefore performs two `arity()` borrows on every one of those comparisons and
then performs two more mapped `arguments()` borrows on more than 98% of them.

Other local costs are smaller or less safely removable:

- cached standard-weight classification is attributed 26,010,180 instructions;
- free- or de-Bruijn-variable type comparison accounts for about 17,079,315
  instructions across its entry, option handling, structural comparison, and
  drop lines;
- nullable-slot zipping and the recursive-call line each account for
  7,638,546 exclusive instructions.

The C implementation reads `arity` and `args` directly from the same term
cell. Rust can recover the same access shape safely by borrowing each argument
slice once, comparing the slice lengths, and retaining those immutable borrows
for the existing recursive traversal. This removes 711,744 separate `arity()`
borrows while adding early argument borrows only for the roughly 5,643
unequal-arity comparisons.

## Result

Diagnostic only; production source is unchanged.

The next bounded candidate is to move the two existing `arguments()` borrows
before the arity comparison and derive arity from the borrowed slice lengths.
It must preserve the current normalized `-1`/`0`/`1` return values and isolate
this borrow reuse from property caching or other comparator changes.

The raw profile is preserved at:

```text
.artifacts/experiments/2026-07-23-013-struct-weight-line-attribution/rust-callgrind-struct-weight-lines.out
```

Reproduction:

```bash
callgrind_annotate --auto=yes --inclusive=no --threshold=0.01 \
  .artifacts/experiments/2026-07-23-013-struct-weight-line-attribution/rust-callgrind-struct-weight-lines.out \
  src/terms/termfunc.rs

callgrind_annotate --auto=yes --inclusive=no --threshold=0.01 \
  .artifacts/experiments/2026-07-23-013-struct-weight-line-attribution/rust-callgrind-struct-weight-lines.out \
  src/terms/termtypes.rs
```
