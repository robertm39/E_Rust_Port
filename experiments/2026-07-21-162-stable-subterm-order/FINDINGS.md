# Stable subterm payload order

## Question

Can the intermittent `LCL365-1.p` proof-order mismatch be removed without
changing term identity or materially reopening the one-second LUSK6 and
BOO/SWV resource boundaries?

## Diagnosis

The accepted experiment-160 source produced the C proof in 19 of 20 direct
native LCL runs and an alternate valid proof once. C remained stable in the
sample. The two proofs shared all input and early generated clauses, then
reversed the order of two superposition batches at `c_0_17`.

Both implementations order `SubtermOcc` payloads by shared-term identity. C's
`CmpSubtermCells()` compares raw pointers allocated through its fixed-size
memory machinery. Rust compared independently allocated `Rc` addresses. The
identity relation was correct, but the Windows allocator occasionally changed
the relative address order between processes and therefore changed the order
in which otherwise equivalent indexed occurrences were visited.

## Change

`SubtermOcc` now captures the shared term's stable `entry_no` and orders cells
by `(entry_no, identity)`. The entry number is a stable surrogate for C's term
allocation order, while the identity tie-break preserves distinct cells and
exact lookup semantics. The standalone `SubtermTree` uses the same composite
key. A regression checks entry-number ordering and same-entry identity
separation.

The captured number avoids borrowing the term cell on every splay comparison.
No unsafe code, dependency, or global identifier is introduced.

## Proof-order result

The candidate produced the C-normalized proof hash
`642DED3F729C57996868EC16432BC2538FE25A35908506392C201E238387F878` in 20 of
20 direct native runs. Six additional focused C/Rust harness repetitions were
all exact:

- `.artifacts/e-compare/20260721-023543-517832/`
- `.artifacts/e-compare/20260721-023548-030949/`
- `.artifacts/e-compare/20260721-023551-696953/`
- `.artifacts/e-compare/20260721-023555-017406/`
- `.artifacts/e-compare/20260721-023558-576998/`
- `.artifacts/e-compare/20260721-023601-959023/`

The final 50-case report
`.artifacts/e-compare/20260721-025009-824465/` has zero unexpected mismatches
and one declared difference. LCL365 is exact (`Unsatisfiable`, exit 0, equal
normalized proof); the only unequal normalized output is the already declared
higher-order `sledgehammer.p` proof.

## Performance and resource result

The retained profile is
`.artifacts/experiments/2026-07-21-162-stable-subterm-order/rust-callgrind-stable-subterm.out`.
It preserves the LUSK6 proof outcome at 12,889,454,347 instructions, 1,003,223
above the accepted 12,888,451,124 baseline (+0.0078%). The C reference remains
5,254,361,329 instructions, so the rounded C/Rust ratio remains 2.453. The
small deterministic cost is accepted in exchange for process-stable inference
order.

Focused proof report `.artifacts/e-compare/20260721-024110-268691/` is exact
for GEO288, HEN011, LUSK6, and LUSK6ext. Focused resource report
`.artifacts/e-compare/20260721-024259-852021/` is exact for BOO020 and SWV851;
both implementations return normalized `ResourceOut` with exit 8.

The one-second LUSK6 boundary remains narrow. A direct candidate sample run
immediately while an orphaned resource worker retained about 1.8 GiB produced
four `ResourceOut` results and one proof. After that worker exited, the repeat
produced five proofs in five runs. The authoritative matrix sequence proves
exactly: C takes 0.387 seconds and Rust 0.970 seconds. BOO020 and SWV851 in that
same matrix remain exact `ResourceOut`, at 60.892 and 60.255 Rust wall seconds
respectively.

## Validation

- `cargo fmt --all -- --check`
- 4,375 library tests plus all integration targets and features
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four documentation gates
- clean vendored C worktree

## Decision

Accept stable entry-number-first subterm payload ordering. It closes the last
unexpected output row in the maintained matrix at a measured 0.0078%
instruction cost. The broader performance issue remains open because the Rust
profile is still 2.453 times the C reference and the one-second cutoff retains
little load margin.
