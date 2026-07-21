# Fallible clause-insertion reservation at the memory boundary

## Question

Can the intermittent native-Windows `BOO020-1.p` allocator abort be removed
without increasing the translated 2-GiB C data allowance, moving millions of
live clauses, or regressing exact proof behavior?

## Setup

- Parent source: commit `e20f057e` (`Record rejected inline KBO balance
  stacks`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Native resource boundary: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Full compatibility inventory: 50 main-executable cases with the one existing
  declared `sledgehammer.p` normalized-output difference.

Raw comparison reports are retained under `.artifacts/e-compare/`. The direct
three-run before/after samples are retained beside this report.

## Diagnosis and rejected candidates

Changing the sparse clause page from 1,024 to 512 headers only changed the
failing request from 139,264 to 69,632 bytes. BOO still aborted in
`.artifacts/e-compare/20260721-053241-385442/`, so allocation quantum alone was
not the cause.

Reserving a complete clause page before evaluation indexing also remained
insufficient. BOO aborted on 139,264 bytes in
`.artifacts/e-compare/20260721-054107-455957/`. Temporary diagnostics then
identified the large owner as the five-evaluation unprocessed set: near the
limit it held about 1.70 million live clauses in 1.77 million slots. Only about
4% of those slots were holes. Moving 1.7 million live headers to reclaim that
small fraction would repeat the broad compaction approach that previously made
HEN miss its proof boundary.

The early-page candidate did prove that page reservation moved the next cliff:
one diagnostic run failed on 196,608 bytes while growing an evaluation-index
node arena. A first headroom-stop version returned exact `ResourceOut` in
`.artifacts/e-compare/20260721-060705-863468/`, but the three-run sample in
`headroom-stop-boo.csv` still has one 196,608-byte allocator abort and only two
normal exits. A page-only guard therefore remained nondeterministic.

## Accepted design

On non-Linux targets, `ClauseSet::insert` now reserves every vector capacity
that the evaluated-clause insertion can grow before it mutates the indexes:

- the complete next sparse clause page and its outer page handle;
- one node in every evaluation splay-index arena; and
- the evaluation-object slot map.

These reservations use `Vec::try_reserve`/`try_reserve_exact`, so a rejected
allocation becomes an ordinary result instead of invoking Rust's infallible
allocation failure handler. With an active cooperative CPU limit, failure
latches that configured deadline and lets saturation leave through the same
hard-limit `ResourceOut` path used elsewhere. Windows also stops before an
indivisible clause allocation when private-commit headroom falls within two
complete page quanta. The ordinary one-second CPU lookahead remains unchanged,
and Linux keeps its asynchronous `SIGXCPU` path and exact hot insertion code.

The reservation methods may retain a successfully reserved earlier vector if
a later reservation fails. That is intentional: the time latch stops the
search immediately, and retaining capacity avoids a second allocation attempt
during shutdown. No clause, evaluation object, or index entry is partially
inserted.

## Results

The accepted direct sample in `fallible-reserve-boo.csv` returns exit 8 and
`ResourceOut` in all three runs. Sampled peaks are 2,480,316--2,480,888 KiB;
all stderr values are the normal CPU-limit diagnostic.

- Resource report `.artifacts/e-compare/20260721-062222-501381/`: BOO020 and
  SWV851, zero mismatches.
- Proof report `.artifacts/e-compare/20260721-062633-802726/`: GEO288, HEN011,
  LUSK6, and LUSK6ext, zero mismatches.
- Full report `.artifacts/e-compare/20260721-062825-333906/`: 50 cases, zero
  unexpected mismatches, and the one declared `sledgehammer.p` difference.

The full report also passes the synthetic one-second LUSK case that remained
load-sensitive in recent slices. No new Callgrind profile is needed for this
candidate because all `ClauseSet` reservation calls are compile-time excluded
from the Linux benchmark path; the accepted Linux search code is unchanged.

## Validation

- `cargo fmt --all -- --check`
- 4,378 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean unchanged vendored C checkout

## Decision

Accept fallible pre-reservation across the complete evaluated-clause insertion
and immediate active-deadline latching at exhausted Windows headroom. Reject
512-header pages, page-only early reservation, and low-density global
compaction. The accepted candidate makes the formerly intermittent BOO boundary
repeatable without changing the memory-limit translation or Linux hot path.
Whole-prover performance parity remains incomplete at the last measured 2.432
Callgrind ratio, so the main performance Bead stays open.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-165-headroom-stop
& .\experiments\2026-07-19-134-compact-clause-owners\measure_windows.ps1 `
  -Binary .\target\native-165-headroom-stop\release\eprover.exe `
  -Problem .\.artifacts\e-corpus\diversity-scratch-139\resource_boo\BOO020-1.p `
  -OutputCsv .\experiments\2026-07-21-165-clause-page-headroom\fallible-reserve-boo.csv `
  -Label fallible-reserve -Runs 3 -CpuLimit 60
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-165-headroom-stop\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-165-headroom-stop\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
