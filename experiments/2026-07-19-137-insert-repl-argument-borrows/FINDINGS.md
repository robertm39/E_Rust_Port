# Replacement insertion argument borrows

## Question

Can `TermBank::insert_repl` follow C's direct source/destination argument-array
walk and avoid cloning one Rust argument vector plus re-borrowing the
destination `RefCell` for every child?

## Setup

- Parent commit: `694e046a`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Windows candidate:
  `target/insert-repl-borrows/release/eprover.exe`, SHA-256
  `1FBBACF9BAFFB6A9A56456598D7969561891DDA40B2E08065CE67876995DD79B`.
- Deterministic profile: unchanged LUSK6 fixture under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Maintained comparison: native Windows Rust versus archived WSL C with the
  standard 50-case harness.

The pre-candidate profile from experiment 136 contains 18,110,999,911
instructions. `EqnList::copy_repl` owns 2,943,673,558 instructions, or 16.25%
of the profile, almost entirely through two `TermBank::insert_repl` calls per
literal. Each non-variable recursive node clones its complete
`Vec<Option<Term>>`, drops that temporary iterator, and calls
`set_argument()` with a new mutable `RefCell` borrow for every destination
child.

## Source comparison

C `TBInsertRepl()` allocates one unshared top cell and walks its source and
destination flexible argument arrays directly in the same recursive loop. C
`TBInsertReplPlain()` uses the same walk while tracking whether any child
changed.

Rust now exposes one crate-private mutable argument-slice borrow on `Term`.
Both replacement insertion functions borrow the source arguments once, borrow
the new unshared destination arguments once, recurse in the unchanged index
order, and assign each completed child directly. The destination cannot alias
the live term bank because `term_top_insert()` still runs only after the
mutable slice borrow is dropped.

## Deterministic result

The candidate profile contains 17,671,438,618 instructions, a reduction of
439,561,293 instructions or 2.43%. The exact proof and relevant call counts are
unchanged.

| Profile owner | Before | After | Change |
| --- | ---: | ---: | ---: |
| Program total | 18,110,999,911 | 17,671,438,618 | -2.43% |
| `EqnList::copy_repl` | 2,943,673,558 | 2,508,513,371 | -14.78% |
| `Eqn::copy_repl` | 2,927,787,987 | 2,492,149,211 | -14.88% |
| `TermBank::insert_repl` | 2,905,696,394 | 2,470,057,618 | -14.99% |

The retained profile is
`.artifacts/experiments/2026-07-19-137-insert-repl-argument-borrows/callgrind-current.out`.
Together with experiment 136, deterministic LUSK6 instructions have fallen
11.20% from the retained 19,899,749,157-instruction baseline.

## Compatibility result

The focused LUSK6 report at
`.artifacts/e-compare/20260719-120458-276850/` is exact. The final maintained
report is `.artifacts/e-compare/20260719-120601-246989/`: 50 cases, one
unexpected mismatch, and the declared `sledgehammer.p` proof-text difference.

- BOO020 and SWV851 remain exact `ResourceOut`/8 cases.
- GEO288 proves with exact output in 11.64 seconds.
- HEN011 proves with exact output in 60.62 seconds.
- LUSK6 and `LUSK6ext` prove with exact output in 3.20 and 7.79 seconds.
- The synthetic 16 MiB memory-limit case remains exact.
- The sole unexpected case is synthetic one-second LUSK6: C proves in 0.46
  seconds, while Rust reaches `ResourceOut` at 1.07 seconds.

The candidate therefore restores exact unlimited LUSK6 proof ancestry in the
maintained matrix, but it does not close the remaining roughly 2.3x cutoff
gap.

## Falsification checks

- Existing replacement tests cover ordinary instantiated replacement, plain
  changed/no-change behavior, property clearing, and applied-variable prefix
  dereferencing; both focused tests pass.
- `cargo check --locked --all-targets --all-features` passes before profiling.
- Callgrind records the identical proof and call counts while removing the
  expected clone/drop and per-child borrow subtree.
- The full matrix exercises first-order, higher-order, resource, syntax,
  stdin, proof-documentation, and small-memory paths rather than relying only
  on LUSK6.
- The vendored C checkout is not modified.

## Decision

Accept the direct argument-slice walk. It matches the C owner and recursion
order more closely, removes 2.43% of deterministic LUSK6 instructions, and
improves the maintained matrix to one unexpected case without weakening
resource behavior. Continue profiling the remaining rewrite and PD-tree
owners; the one-second LUSK6 acceptance criterion remains open.
