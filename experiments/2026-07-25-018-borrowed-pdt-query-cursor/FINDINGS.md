# Experiment 319: Borrowed PDTree query cursor

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the first-order PDTree matcher keep non-owning query cursors like C while
parking reference-counted owners whenever control returns through the stateful
safe search API?

## Baseline

- Accepted parent: commit `05710c73`.
- The freshly rebuilt parent retires `7,690,652,762` instructions on the
  matched LUSK6 profile, versus `5,254,418,333` for C (`1.463654x`).
- `PdTree::search_next_matching_occurrence_impl::<true>` retires
  `1,561,232,620` self instructions for `783,453` calls and exposes
  `1,651,582,089` caller-inclusive instructions.
- Rust's stateful cursor stores every pending and processed query cell as an
  owned `Term`. Symbol traversal clones all children, backtracking moves and
  drops those owners, and variable traversal moves another owner. C stores raw
  `Term_p` values in its two traversal stacks.

## Candidate

First-order lookup now uses a separate `PdtBorrowedSubstCursor`. Its pending
query stack and processed-query stack contain crate-private
`BorrowedTermCell` cursors. Higher-order lookup retains the existing owned
cursor without entering this path.

The stateful safe API makes a non-owning cursor more demanding than a
synchronous walker: the caller can retain the query, replace one of its
arguments after a match, release the detached child, and then request the next
match. A guard therefore rebuilds a parked owner vector whenever a search call
returns or unwinds. The replacement owners are acquired before the previous
set is released. The active search state already owns the exact root cursor,
so only descendant cursors are parked; a detached descendant consequently
remains live until it is consumed or search exit resets the cursor.

The borrowed path directly:

- reads function code for symbol alternatives;
- borrows the argument slice once to push initialized children in reverse
  order;
- copies type UID through a scoped `RefCell` borrow without cloning the type
  owner;
- uses cached weight for shared cells and the fixed free-variable weight; and
- temporarily reconstructs an owner only for an unshared compound structural
  weight fallback or for an accepted substitution binding.

All argument and type access retains safe dynamic-borrow conflict detection.
The cursor never bypasses an active mutable `RefCell` guard. Search exit resets
both cursor implementations before recycling the active query state.

The private unsafe contract is:

- every raw cursor originates from `Rc::as_ptr` provenance exposed by
  `Term::borrowed_cell`;
- on entry, the active root or the previous parked-owner vector owns every
  cursor allocation;
- a newly pushed child remains owned by its still-live parent until the guard
  parks the complete continuation;
- replacement parked owners are acquired before old parked owners are
  released;
- no user callback or structural mutation occurs inside one search call; and
- all raw cursor fields are cleared before the search root and parked owners
  are released.

Focused regressions cover first-order stack order and cursor state, ordinary
matching/backtracking/type/weight behavior, higher-order fallback, and the
mutation-between-calls liveness boundary. The latter detaches a matched child,
drops the external owner, and proves that the next specific match still
completes from the parked cursor.

## Setup and exact commands

Final focused validation and measurement used dedicated worker
`e-rust-codex-260726-084546-1ea3` with Rust 1.97.1 and Valgrind 3.22.0. Its
final uploaded worktree snapshot SHA-256 was
`d9f7044c2516b8c19d0685f0f574603438a65c4d0eb820fe2075f186fe364c7d`.
The accepted parent archive SHA-256 was
`E75D11FF8D042EDD3E90F5CDA93A5B1A6E960745B5BC563268C8180605D095B5`.
Measured candidate production-file SHA-256 values were:

```text
fd7bff277d5ab49b35dcb787d75927aa98015f4bbb5b143dec217acc85cac703  src/clauses/pdtrees.rs
0184c3418baee3d5f20bff35ca6e1d357c3cbadc1afa03df6390dc4d02b44b4c  src/terms/termtypes.rs
```

The focused scripts preserve the exact Rustfmt, 200 focused tests, strict
all-feature library pedantic Clippy, parent/candidate release builds,
Callgrind commands, proof comparisons, and two independent 64-pair native
commands. The final controller lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 05710c73 `
  src/clauses/pdtrees.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-018-borrowed-pdt-query-cursor/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-319-refined2
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-018-borrowed-pdt-query-cursor/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-319-refined2
}
finally {
    .\linode-runner.ps1 down
}
```

Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-091550-b31e` and snapshot
`ac97cd9bba2c74e4ce07768978828bc4013844ba53a35c2c4f7bee5d2e9e0c68`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- First-order traversal order, repeated-variable identity, type/weight
  constraints, live substitution bindings, terminal order, and backtracking
  must remain exact.
- Every non-owning cursor must retain valid `Rc::as_ptr` provenance and remain
  live across normal returns, exhausted searches, panics, query mutation, and
  search exit.
- Active mutable argument/type guards must retain safe `RefCell` panic
  behavior rather than being bypassed.
- Higher-order traversal must retain the existing owned cursor.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact work must improve materially at the intended PDTree owner, and
  repeated alternating native timing must confirm the production direction.

All workers and firewalls were deleted after artifact collection or an
early failed gate.

## Results

Focused Rustfmt, all 44 PDTree tests, all 19 term-cell tests, all 12
substitution tests, all 125 term-bank tests, and strict all-feature library
pedantic Clippy pass: 200 focused tests in total.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `7,690,652,762` to
`7,662,328,035`, a reduction of `28,324,727` (`0.368301%`). Relative to the
matched C count of `5,254,418,333`, the candidate ratio is `1.458264x`. The
release executable grows by 5,872 bytes (`0.0710%`), from 8,265,288 to
8,271,160 bytes.

The intended owner improves more strongly. The first-order search
specialization falls from `1,561,232,620` to `1,514,251,396` self
instructions, a reduction of `46,981,224` (`3.009239%`). Its inclusive cost
falls from `1,651,582,089` to `1,604,529,380`, a reduction of `47,052,709`
(`2.848948%`). The new return-boundary guard costs `115,967,853`
instructions, including `20,252,029` in vector drop glue, across `880,523`
unwinds/returns. That explicit liveness cost is included in both the owner and
whole-program results.

Two independent native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 125 pairs, and every run has the exact proof hash. Across all
pairs:

- wall mean, median, paired mean, and paired median improve by `2.679130%`,
  `2.813690%`, `2.670546%`, and `2.753865%`;
- CPU mean, median, paired mean, and paired median improve by `2.680838%`,
  `2.813248%`, `2.672254%`, and `2.751753%`.

Restricting both blocks to their final halves yields 64 pairs and 62 wins:

- wall mean, median, paired mean, and paired median improve by `2.770997%`,
  `2.826574%`, `2.762812%`, and `2.684593%`;
- CPU mean, median, paired mean, and paired median improve by `2.772519%`,
  `2.843722%`, `2.764352%`, and `2.697574%`.

The independent blocks agree. Block one records 62/64 wins and paired mean
wall/CPU improvements of `2.580873%`/`2.582542%`; block two records 63/64
wins and improvements of `2.760218%`/`2.761966%`.

Raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-018-borrowed-pdt-query-cursor/experiment-319-refined2/
```

The retained focused archive is
`.artifacts/experiments/2026-07-25-018-borrowed-pdt-query-cursor/remote-refined2.tar.gz`
with SHA-256
`26B18B4B4456F405BFBDC731EFBBD34A64A6FFC7DB191BC83E9AEE87067BB7AC`.

Fresh comprehensive run `.artifacts/linode/260726-091550-b31e/` validates the
exact focused candidate executable SHA-256
`b3448a5684c9dc3723828b7537175bcf674470937c38bc9d516b280c122524d8`:

- 4,416 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order pinned-C references build and pass
  smoke checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records `9,905,696` Rust versus `7,590,630` C instructions.

The fresh aggregate is `1.1411223971x` Rust/C wall time. Experiment 318's
fresh aggregate was `1.1289063169x`; this cross-worker reversal is not used as
causal evidence. The same-worker deterministic and 128-pair alternating
measurements establish the candidate's direction. `VALIDATION_COMPLETE` and
`SUCCESS` both contain `ok`.

## Falsification checks and limits

- The mutation-between-calls regression explicitly forces first-order
  dispatch, observes the root and descendant cursors, confirms that only the
  descendant is parked, replaces the root argument, drops the detached
  external owner, and completes the next exact match.
- Root-owner elision is pointer-exact. The active `PdtSearchState.term` owns
  that allocation until both cursors reset; every descendant is independently
  parked even if its parent is the root.
- The RAII guard parks the continuation on normal match returns, exhausted
  returns, and unwinding. The double-buffered owner vectors prevent a
  replacement query shape from releasing an old cursor before its new owner
  is acquired.
- Type and argument helpers use scoped safe `RefCell` borrows. Existing
  mutable-guard conflict detection is retained.
- Higher-order specialization never dispatches to the borrowed cursor.
- Preliminary worker `e-rust-codex-260726-082007-b234` measured the safer
  version that also parked the already-owned root. It improved exact work by
  `0.135366%` and combined paired mean wall/CPU time by
  `1.737950%`/`1.737737%`. Those retained results motivated the pointer-exact
  root-owner refinement but are not substituted for the final evidence. Raw
  data and archive are retained as `experiment-319/` and `remote.tar.gz`
  (SHA-256
  `D14FAFE06FA6D2328EA9ADF683A7DE610B4C23C55EE3CFD1A153293ACDC7401E`).
- Earlier source snapshots stopped at provisioning, Rustfmt, compile, or
  focused-test failures before performance measurement. One focused liveness
  test initially followed the global higher-order mode; the final regression
  explicitly selects first-order dispatch. All reported final evidence comes
  from the corrected implementation.
- The first comprehensive worker
  `e-rust-codex-260726-091116-2665` stopped at strict test-target Clippy because
  its new fixture named adjacent bindings `types` and `type_`. Renaming only
  that `#[cfg(test)]` binding to `default_type` changed the final
  `src/clauses/pdtrees.rs` source SHA-256 to
  `65439d1b5e4cc8c8e2f3dd0197dc1c369a059b043a5079c40849b7e243ba7378`.
  The final comprehensive release binary is byte-identical to the measured
  candidate, confirming that the test-only rename did not change production
  code. The failed worker and firewall were deleted automatically.
- The aggregate remains above `1.10x`; this closes one measured
  reference-counting differential, not the performance epic.

## Decision

Accept. The unsafe scope is private and its stateful lifetime contract is
explicitly tested at the mutation-between-calls boundary. The intended owner
falls 3.01%, exact whole-program work falls 0.368%, both independent native
blocks improve by more than 2.5%, the exact proof is preserved, and the
complete compatibility, resource, portability, and quality matrices remain
green. Main-prover performance parity remains open.
