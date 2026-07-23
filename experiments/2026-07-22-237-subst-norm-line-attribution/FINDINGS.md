# Experiment 237: Substitution-normalization line attribution

## Question

Which operations dominate the current accepted `Substitution::norm_term`
after the later dereference inlining and ownership changes, and what bounded
candidate remains untested against the 437,245,456-instruction normalizer?

## Baseline

- Source: commit `7f0f87f6`, whose executable source remains accepted
  Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- `Substitution::norm_term`: 437,245,456 exclusive instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- C `SubstNormTerm`: 192,675,144 exclusive instructions.
- Rust/C whole-program ratio: 1.888634.

## Profile

The accepted source was rebuilt with release optimization and line tables:

```text
CARGO_PROFILE_RELEASE_DEBUG=1
CARGO_TARGET_DIR=target-wsl-237-subst-norm-lines
cargo build --release --locked --bin eprover
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out \
  target-wsl-237-subst-norm-lines/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

The profiled binary proved the problem and retired 9,923,799,136
instructions. That is 234,364 instructions, or 0.002362%, above the accepted
baseline, so the line-table build is representative.

`callgrind_annotate` attributes all 437,245,456 exclusive instructions in
`Substitution::norm_term` as follows:

| Source attribution | Instructions |
| --- | ---: |
| `src/terms/termtypes.rs` | 79,545,592 |
| `core::mem` | 70,749,970 |
| `alloc::vec` | 63,867,333 |
| `src/terms/subst.rs` | 40,408,018 |
| `core::cell` | 40,332,017 |
| `core::option` | 37,967,143 |
| `core::intrinsics` | 20,569,251 |
| `core::ptr::non_null` | 17,845,561 |
| `alloc::rc` | 13,683,929 |
| integer macros | 12,570,354 |
| `alloc::raw_vec` | 12,453,304 |
| `core::ptr` | 12,453,304 |
| slice iteration | 9,062,386 |
| iterator flattening | 5,737,294 |

The traversal performs 3,260,660 nonvariable visits. On every one, the
general Rust always-dereference path tests whether the term is an applied
free variable:

| Dereference operation | Instructions |
| --- | ---: |
| Initial non-free-variable test | 22,666,047 |
| `is_applied_free_var()` line, exclusive | 32,077,471 |
| Calls made by `is_applied_free_var()` | 35,867,260 |
| Applied-variable test subtotal | 67,944,731 |

The applied-variable subtotal alone is 15.54% of the normalizer and 0.685% of
the whole profile. Including the preceding branch gives a 90,610,778
instruction upper bound for the area affected by a first-order
specialization; it is not an estimate that all of those instructions can be
removed.

The original C normalizer selects its dereference function once per
`SubstNormTerm` invocation:

```c
Term_p (*deref)(Term_p) =
   problemType == PROBLEM_HO ? WHNF_deref : TermDerefAlways;
```

The maintained performance reference is C's non-LFHO build, where
`TermDerefAlways` follows only the ordinary binding chain. C's separately
archived LFHO build compiles an applied-free-variable test into that helper,
but valid first-order terms do not contain that shape. The unified Rust
normalizer calls its applied-variable-capable `term_deref_always` for every
popped term, including all 3,260,660 nonvariables in this first-order LUSK6
run.

## Result

This was a diagnostic experiment; production source is unchanged.

The next bounded candidate is a problem-mode dispatch once per normalization
call: retain the existing higher-order path for higher-order problems, but
use a first-order always-dereference helper that only follows free-variable
bindings for first-order problems. This matches the original C algorithm and
removes the applied-free-variable question from the first-order traversal.

This candidate is distinct from the rejected general owned-dereference
change (Experiment 167), the accepted `Always`-mode specialization
(Experiment 185), direct nonvariable handling inside the general
higher-order-capable path (Experiment 190), dereference-helper inlining
(Experiments 199 and 200), and forced wrapper inlining (Experiment 221).

Raw Callgrind data is preserved at:

```text
.artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out
```
