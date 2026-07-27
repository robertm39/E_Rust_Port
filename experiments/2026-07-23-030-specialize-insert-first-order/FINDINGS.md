# Experiment 268: Reject first-order ordinary-insertion specialization

## Status

Rejected in Experiment 268 for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the successful first-order specialization from Experiment 267 also remove
inert LFHO dereference work from `TermBank::insert_with_mode` and its ordinary,
keep-variable, and no-property insertion callers?

## Candidate

The public `insert_with_mode` entry read the problem type once and dispatched
exactly `ProblemType::FirstOrder` to a const-specialized recursive helper.
That helper called `term_deref_if_changed` directly, used a zero applied-prefix
limit, and passed the current dereference mode unchanged to children.
`HigherOrder` and `NotInitialized` retained the existing general helper and
`convert_lfho_deref` path.

The three insertion modes remained a runtime enum. The candidate therefore
tested only the first-order LFHO specialization and did not combine it with a
mode-representation change.

A focused first-order regression followed a bound variable in a recursive
ordinary insertion. The existing LFHO regression continued to exercise
ordinary, keep-variable, and no-property insertion through applied-variable
prefix expansion.

## Baseline and validation

Accepted Experiment 267:

- Rust instructions: 9,024,090,576
- C instructions: 5,254,361,329
- Rust/C ratio: 1.717448

Before measurement:

- both focused recursive-insertion tests pass with default and all features;
- strict all-feature library pedantic Clippy passes;
- the WSL default-feature release build succeeds; and
- direct LUSK6 execution proves `Unsatisfiable` and exits zero.

## Deterministic result

The candidate preserves the exact proof but retires 9,129,449,367
instructions:

- global delta: +105,358,791;
- global regression: +1.167528%;
- hypothetical Rust/C ratio: 1.737499.

The intended insertion boundary improves:

| Exclusive owner | Accepted | Candidate | Change |
| --- | ---: | ---: | ---: |
| insertion recursion | 72,734,723 | 112,442,212 | +39,707,489 |
| general changed-only root dereference | 45,705,727 | 0 | -45,705,727 |
| aggregate | 118,440,450 | 112,442,212 | -5,998,238 (-5.064349%) |

The global result reverses because the additional const clone changes inlining
elsewhere. Accepted `Substitution::norm_term` accounts for 437,245,456
exclusive instructions with its always-dereference loop inlined. In the
candidate, `norm_term` falls to 302,975,103 but
`term_deref_always` reappears out of line at 276,328,019, for an aggregate
579,303,122 instructions: +142,057,666 or +32.489226%. Smaller reductions in
`TermTree::insert`, `term_top_insert`, and the PD-tree cursor do not offset
that code-generation regression.

This is the same kind of whole-program inlining sensitivity seen in earlier
micro-specialization experiments. A local intended-owner win is not sufficient
when the optimized production binary moves a hotter loop out of line.

## Decision

Reject. Restore the accepted Experiment 267 source and tests byte-for-byte.
Native timing and compatibility/resource matrices are skipped because the
exact deterministic workload and its whole-program owner attribution are
decisively negative.

The raw rejected profile is retained at:

```text
.artifacts/experiments/2026-07-23-030-specialize-insert-first-order/rust-callgrind-specialize-insert-first-order.out
```

## Falsification checks

- The focused first-order test forces the proposed branch and passes.
- The existing LFHO test passes on the unchanged general branch for all three
  insertion modes.
- Direct and Callgrind runs preserve the exact theorem outcome.
- Intended-owner attribution confirms the optimization itself removes work.
- Whole-program attribution identifies the larger out-of-line
  substitution-dereference regression rather than treating the global count as
  unexplained noise.
- After rejection, `git diff -- src/terms/termbanks.rs` is empty.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-specialize-insert-first-order.out \
  target-wsl-268-insert-fo/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
