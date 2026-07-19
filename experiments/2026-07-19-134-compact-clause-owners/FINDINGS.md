# Compact clause owners and streamed SAT import

## Question

Can C-shaped nullable-owner, handle, clause-store, and SAT-import
representations reduce the native Windows live-memory deficit enough for the
maintained resource-bound main cases to terminate through E's configured
resource path instead of aborting inside the Rust allocator?

## Baseline

The production baseline is commit `9aac4a20`. Its native Windows binary has
SHA-256
`E8A6FEC84F0A1D8AB4822AF4003C290F41133F31B16FC73CDC92846ACE7BF093`.
The accepted candidate binary has SHA-256
`C740DCA46C90BA2FBECB6270D3D1BA06B2D69473759705788577D7BC193C8333`.
Both use the maintained `--auto --silent --cpu-limit=60
--memory-limit=2048 --detsort-rw --detsort-new --proof-object=1` surface.

The baseline clause header is 192 bytes on the 64-bit target. A missing
derivation still occupies a 32-byte inline `Option<PStack<_>>`, nullable
`usize` handles occupy 16 bytes, and the complete sparse owner is one
power-of-two `Vec<Option<Clause>>`. BOO020 therefore fails while replacing a
192 MiB clause buffer with a 384 MiB buffer. SWV851 fails on the same general
allocator path. The maintained 50-case report records both as exit 9 with no
SZS status, while C reports `ResourceOut`/8.

## Candidate sequence

The accepted layout changes are individually C-shaped:

- `Clause::derivation` is a nullable boxed owner, so absent derivations cost
  one pointer and the clause header is exactly 160 bytes.
- the evaluation object, both evaluation-tree links, and the clause-set
  object-to-slot map encode `handle + 1` in `NonZeroUsize`, preserving zero as
  `None` without doubling nullable-handle size and reducing `SimpleEvalCell`
  from 48 to 32 bytes;
- evaluation object handles remain monotonic after extraction, preserving the
  final `EvalCompare` insertion-order tie-break; and
- the sparse clause owner keeps its first page inline and adds lazily grown
  overflow pages of at most 4,096 headers. Small sets retain ordinary `Vec`
  growth, while large sets avoid whole-owner slack and hundreds-of-megabytes
  replacement allocations. Stable numeric slots encode page and offset;
  sorting/compaction rebuild all dependent maps.

[`boo020-layout-only.csv`](boo020-layout-only.csv) shows that the initial fully
packed header lowered BOO020 sampled peak RSS from 1,845,028 to 1,732,668 KiB
and changed the failed whole-store allocation from 402,653,184 to 335,544,320
bytes, but did not remove the abort. The rejected handle-reuse intermediate in
[`boo020-reused-handles.csv`](boo020-reused-handles.csv) ran about five more
CPU seconds before a small allocation failed. It was removed after the full
matrix showed that recycled handles changed selection tie order. Paged storage
removes the growth cliff without recycling identities.

Selective one-link and fully packed intermediates did not stabilize the
allocator-sensitive `LUSK6ext` proof. The fully packed report at
`.artifacts/e-compare/20260719-065153-159888/` differs from C, while a focused
one-link report at `.artifacts/e-compare/20260719-072444-335026/` and the final
fully packed matrix are exact; repeated runs of both layouts switched between
the two proof ancestries. The robust fully packed layout is retained because
it provides the largest resource margin. The proof-order sensitivity remains
open rather than being classified as an expected difference.

The page candidate exposed one more Rust-only owner in SWV851. A symbolized
optimized backtrace in
[`swv851-paged-symbolized.csv`](swv851-paged-symbolized.csv) resolves the
83,886,080-byte allocation to `proof_state_sat_check`. Rust cloned all five
live processed/unprocessed sets into one `Vec<Clause>` before SAT import; C
streams those sets directly through one pseudo-grounding substitution. Rust
now borrows the term bank mutably alongside the five disjoint set fields and
imports them in C order without a whole-state clone.

Windows has no `RLIMIT_CPU` signal, so its hard limit is cooperatively polled.
A long SAT import could previously run past the deadline and exhaust memory
before returning to the saturation loop. Main proof-control SAT import now
polls between clauses and returns without a report when the configured limit
fires; the next saturation condition renders the existing hard-limit
`ResourceOut` path. Standalone SAT APIs keep their uninterruptible behavior.

## Resource results

The accepted candidate closes both allocator failures:

| Problem | Baseline | Accepted candidate | Candidate peak RSS |
| --- | --- | --- | ---: |
| BOO020-1.p | allocator abort, exit 9, no SZS | `ResourceOut`, exit 8 | 1,914,504 KiB |
| SWV851-1.p | allocator abort, exit 9, no SZS | `ResourceOut`, exit 8 | 1,866,984 KiB |

Raw accepted runs are in
[`boo020-hybrid-final.csv`](boo020-hybrid-final.csv) and
[`swv851-hybrid-final.csv`](swv851-hybrid-final.csv). They reach the CPU
boundary at 61.06 and 61.00 measured process seconds, respectively. The
intervening
[`swv851-paged.csv`](swv851-paged.csv) and
[`swv851-streamed.csv`](swv851-streamed.csv) runs retain the evidence that
paging and streaming were each necessary but insufficient alone.

## Throughput falsification

The final candidate and baseline each have three fresh consecutive samples in
[`lusk6-hybrid-final.csv`](lusk6-hybrid-final.csv) and
[`lusk6-hybrid-final-baseline.csv`](lusk6-hybrid-final-baseline.csv). Both
binaries prove every run with identical 10,344-byte output.

| Metric | Baseline median | Candidate median | Change |
| --- | ---: | ---: | ---: |
| Wall time | 3.847770 s | 3.691248 s | -4.07% |
| CPU time | 3.734375 s | 3.625000 s | -2.93% |
| Sampled peak RSS | 247,604 KiB | 240,960 KiB | -6,644 KiB |

The consecutive samples do not support a throughput-improvement claim, but
they falsify a regression in this controlled proof and reduce median peak RSS
by about 6.49 MiB. The exact one-second LUSK6 scenario and 60-second
HEN011 case remain genuine performance failures in
[`lusk6-one-second.csv`](lusk6-one-second.csv) and
[`hen011.csv`](hen011.csv), both returning `ResourceOut`/8 rather than C's
proof outcome. They remain open in `E_Rust_Port-j76.5.3` and must not be
classified as expected differences.

## Validation

- the clause header regression caps the 64-bit inline owner at 160 bytes;
- evaluation-cell tests pin the 32-byte fully packed shape, clause-set tests
  cross the 4,097-clause page boundary, and SAT-interface tests pin
  deterministic interruption;
- the final maintained report at
  `.artifacts/e-compare/20260719-082542-330267/` contains 50 cases, 3
  unexpected differences, and the one declared `sledgehammer.p` proof-text
  difference. BOO020, SWV851, and `LUSK6ext` are exact in this run; the open
  unexpected cases are GEO288, HEN011, and the one-second LUSK6 fixture. The
  baseline had 4 unexpected differences, including both allocator aborts; and
- the vendored C checkout remains unchanged.

## Decision

Accept the compact nullable owners, fully packed monotonic handles, hybrid
paged sparse store, streamed SAT import, and main-proof-control SAT deadline polling. They
replace Rust-only ownership overhead, close both resource-outcome mismatches,
and do not materially regress the controlled proof. Continue the separate
proof-order and throughput work for GEO288, HEN011, one-second LUSK6,
allocator-sensitive `LUSK6ext`, and aggregate C parity.
