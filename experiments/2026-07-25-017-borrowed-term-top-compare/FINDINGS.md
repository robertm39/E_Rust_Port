# Experiment 318: Borrowed term-top comparison

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the term-cell-store splay comparator match C's direct term-top pointer
walk, avoiding two argument `RefCell` guards per comparison while retaining
the accepted owned intrusive links, tree topology, key order, and exact proof
behavior?

## Baseline

- Accepted parent: commit `48957215`.
- Matched LUSK6 parent profile: `7,731,396,395` Rust instructions versus
  `5,254,418,333` C instructions (`1.471409x`).
- The accepted `term_top_insert` owner still retires about 1.0816 billion
  self instructions for 2,479,632 store insertions.
- C compares function code, optional higher-order type address, arity, and
  argument addresses through raw term pointers. Rust already borrows each
  argument slice once per comparison, but still changes two dynamic borrow
  flags at every splay comparison.
- Earlier experiments rejected indexed iteration, direct identity-integer
  ordering, non-owning splay tails, buffered chains, and arena storage. None
  tested a private direct read-only cursor while retaining the accepted splay
  and owning-link implementation.

## Candidate

`term_top_order_for_problem` now enters a private read-only term-cell cursor
while its two owned inputs keep both allocations live. The cursor compares
function code, optional higher-order type identity, arity, and argument
allocation identity directly. It removes the two dynamic argument-borrow flag
updates and the temporary type owners from every splay comparison.

The accepted owning intrusive tree is otherwise unchanged. Splay still moves
the left and right `Cell<Option<Term>>` owners through safe operations, and
the cursor reads neither link. A retained safe owned comparison in the test
module serves as the focused equivalence oracle.

The raw path has a deliberately narrow contract:

- the two owned comparison inputs keep both term cells live for the complete
  synchronous comparison;
- every initialized argument slot contains an owned handle, so the live term
  cells also keep all compared argument allocations live;
- term-tree operations mutate only the disjoint intrusive left/right fields;
- every production term-bank argument guard is dropped before top-tree
  insertion, and no argument or type mutable guard may overlap the cursor;
- term/type metadata is complete and stable before store entry; and
- `Term` and `Type` use `Rc`/`RefCell` ownership and are not sent across
  threads, while the comparator invokes no callback or re-entrant mutation.

The cursor and raw type helper remain crate-private. Safe public term-tree
callers cannot obtain the crate-private mutable argument guard, and ordinary
single-slot construction setters are synchronous. First-order debug builds
retain the matching-type-identity assertion; higher-order comparison retains
type-allocation identity ordering. Missing types and uninitialized arguments
retain their established panics.

## Setup and exact commands

Focused validation and measurement used dedicated worker
`e-rust-codex-260726-072125-ce78` with Rust 1.97.1 and Valgrind 3.22.0. The
final uploaded worktree snapshot SHA-256 was
`73ac89c9419a9eaad21cf4245b906aab21ec3d96de52ba87c0224f9106d886dd`.
The accepted parent archive SHA-256 was
`20B7D703B707A840A1429C981147783C4201D6E8DFF4A83E34049A20DAA2D304`.
Measured candidate production-file SHA-256 values were:

```text
992eb8c0b1f206fc4fe5a6ce49e6a4c4772c7bdc2e6e71bfad28b254383ca1d6  src/terms/termtrees.rs
8f6111e87f62f811996d2b8d95e29a3ff0d1042efbd59e36f2af7a39fc6b69ed  src/terms/termtypes.rs
```

The focused scripts preserve the exact Rustfmt, 157 focused tests, strict
all-feature library pedantic Clippy, parent/candidate release builds,
Callgrind commands, proof comparisons, and two independent 64-pair native
commands. The controller lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 48957215 `
  src/terms/termtrees.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-017-borrowed-term-top-compare/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-318
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-017-borrowed-term-top-compare/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-318
}
finally {
    .\linode-runner.ps1 down
}
```

Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-073856-a680` and snapshot
`0f9577883663d6c4758a24b46b979a5e5f4595da2a536de49593f6a6fde8d61c`:

```powershell
.\linode-runner.ps1 run
```

Both successful workers and firewalls were deleted after artifact collection.

## Falsification criteria

- Function-code, higher-order type identity, arity, argument identity, and
  uninitialized-slot behavior must retain the accepted ordering.
- Both compared top allocations and every initialized argument handle must
  remain live; argument and type metadata must remain immutable and have no
  active mutable guard.
- Splay rotations, root selection, duplicate reuse, insertion accounting, and
  proof output must remain unchanged.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact work must improve materially at the intended term-store owner, and
  repeated alternating native timing must confirm the production direction.

## Results

Focused Rustfmt, all 6 term-tree tests, all 19 term-cell tests, all 7
term-cell-store tests, all 125 term-bank tests, and strict all-feature library
pedantic Clippy pass: 157 focused tests in total.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `7,731,395,459` to
`7,690,646,344`, a reduction of `40,749,115` (`0.527060%`). Relative to the
matched C count of `5,254,418,333`, the candidate ratio is `1.463653x`. The
release executable shrinks by 9,600 bytes, from 8,274,888 to 8,265,288 bytes.

The intended owner explains the global result. Parent
`TermBank::term_top_insert` self cost is `1,081,589,204` instructions. The
candidate insertion plus outlined raw comparator costs `691,409,925` plus
`349,404,830`, or `1,040,814,755` instructions at the same `7,114,571`
comparator calls. That is a reduction of `40,774,449` (`3.769865%`). It
exceeds the whole-program reduction by only 25,334 instructions because
whole-program LTO moves a small amount of work among other owners.

Two independent native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 98 pairs, and every run has the exact proof hash. Across all
pairs:

- wall mean, median, paired mean, and paired median improve by `1.372738%`,
  `1.133708%`, `1.328453%`, and `1.049752%`;
- CPU mean, median, paired mean, and paired median improve by `1.373615%`,
  `1.130814%`, `1.329322%`, and `1.047869%`.

Restricting both blocks to their final halves yields 64 pairs and 50 wins:

- wall mean, median, paired mean, and paired median improve by `1.529819%`,
  `1.295323%`, `1.475789%`, and `0.949678%`;
- CPU mean, median, paired mean, and paired median improve by `1.531151%`,
  `1.304514%`, `1.477032%`, and `0.951037%`.

The independent blocks agree. Block one records 48/64 wins and paired mean
wall/CPU improvements of `1.372710%`/`1.373123%`; block two records 50/64
wins and improvements of `1.284196%`/`1.285521%`.

Raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-017-borrowed-term-top-compare/experiment-318/
```

The retained focused archive is
`.artifacts/experiments/2026-07-25-017-borrowed-term-top-compare/remote.tar.gz`
with SHA-256
`797608D21AA4DBB82D3B7CD8F19843C272C55C676E3401F1AE2DF9ED64F2E59D`.

Fresh comprehensive run `.artifacts/linode/260726-073856-a680/` validates
the exact candidate executable SHA-256
`efb10a17d7f238656ef246e9744f1915b3815dd6aa8afcb98d2c86a1c770125`:

- 4,413 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order pinned-C references build and pass
  smoke checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records `9,899,341` Rust versus `7,590,630` C instructions.

The fresh aggregate is `1.1289063169x` Rust/C wall time. Experiment 317's
fresh aggregate was `1.1332152692x`; this cross-worker difference is not used
as causal evidence. The same-worker deterministic and alternating
measurements establish the candidate's direction. `VALIDATION_COMPLETE` and
`SUCCESS` both contain `ok`.

## Falsification checks and limits

- The cursor and raw type helper are private. The safe tree comparator owns
  both addressed cells for the complete call.
- Every compared argument is owned by an initialized slot in one of those
  live cells. The comparator cannot release an owner or invoke user code.
- The splay path mutates only the separate left/right `Cell` fields while the
  cursor reads function code, type metadata, and arguments. The accepted safe
  owned splay/link implementation is unchanged.
- Production term-bank call sites drop all `arguments_mut` guards before
  `term_top_insert`; the guard is crate-private and unavailable to safe
  external callers. Type metadata is complete and stable at store entry.
- `Rc`/`RefCell` terms and types are single-threaded. No concurrent mutation
  can overlap a comparison.
- The first focused snapshot failed Rustfmt before compilation or
  measurement. The final snapshot contains only the formatter's mechanical
  line wrapping, and all reported performance evidence comes from that final
  candidate.
- The aggregate remains above `1.10x`; this closes one measured
  dynamic-borrow differential, not the performance epic.

## Decision

Accept. The unsafe scope is private, documented, and justified by a measured
C-shaped ownership gap. Focused equivalence covers function, type, arity,
argument, reverse, and equality boundaries; the intended owner falls 3.77%;
exact whole-program work falls 0.527%; both independent native blocks improve;
and the complete compatibility, resource, portability, and quality matrices
remain green. Main-prover performance parity remains open.
