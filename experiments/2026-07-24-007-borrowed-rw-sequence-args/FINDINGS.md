# Experiment 280: Borrowed rewrite-sequence argument pairs

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can structural rewrite-sequence construction recurse over paired borrowed
argument slices, matching C's direct `from->args[i]`/`tmp->args[i]` walk
without cloning two `Rc<TermCell>` handles per child edge?

## Setup

- Parent source: commit `25ba7155` (`perf: reject borrowed occurrence
  traversal`); executable source remains accepted Experiment 270.
- Accepted compact profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: borrow the source and replacement argument arrays for the
  duration of each structural rewrite link and pass child handles by
  reference. Owned top-link traversal and replacement acquisition remain
  unchanged.
- Variant B borrows only the source argument array and retains the accepted
  owned replacement child, testing whether one guard plus one clone transfers
  better than two guards across recursion.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted line profile attributes 163,712,831 instructions to 153,429
top-level `term_compute_rw_sequence` calls and 126,109,659 instructions to
284,810 recursive calls. Original C recurses directly through its two raw
argument arrays.

## Results

### Paired borrowed slices

The candidate preserves the expected unsatisfiable result and produces a
byte-identical native proof object. The intended recursive owner improves in
directly comparable optimized line-table profiles: 284,810 recursive
`term_compute_rw_sequence` calls fall from 126,109,659 to 116,240,734
instructions, a reduction of 9,868,925 or 7.825669%.

The local saving reverses at whole-program scope. Compact instructions rise
from 8,992,812,925 to 9,063,556,057, a regression of 70,743,132 or 0.786663%.
The line-table build likewise rises from 8,994,036,876 to 9,062,705,468
instructions, or 0.763490%. The all-feature native executable grows from
8,952,320 to 9,012,224 bytes.

Holding both argument-array `RefCell` guards across recursive calls therefore
costs more through guard traffic and broader code layout than it saves in
child-handle clone/drop work.

### Source-only borrowed slice

Variant B borrows only the source argument array while retaining the accepted
owned replacement child. It also proves the expected result, but rises further
to 9,073,151,795 instructions: 80,338,870 or 0.893368% above the parent and
9,595,738 instructions above the paired-slice candidate.

## Validation

- All four candidate clause-position tests pass in default and all-feature
  modes.
- The existing recursive rewrite-sequence regression covers a structural root
  link and records the exact injected operation/demodulator stack order.
- Strict all-feature library pedantic Clippy and formatting pass for the
  paired-slice candidate.
- Compact and line-table WSL Callgrind for the paired candidate plus compact
  Callgrind for the source-only variant all prove LUSK6 and exit zero.
- Direct native parent/candidate proof-object output is byte-identical.
- Native timing and compatibility matrices are skipped after both exact
  instruction profiles reject the performance-only change.
- After rejection, accepted `clausepos.rs` is restored byte-for-byte.

## Decision

Reject both variants. Borrowing the paired arrays reduces the recursive
rewrite-sequence owner by 7.825669%, but retaining one or two recursive
`RefCell` guards causes decisive whole-program regressions. Keep Experiment
270 as the accepted baseline at 8,992,812,925 instructions, or 1.711495 times
C.

Rewrite-sequence child ownership is now exhausted at this representation
boundary: cloning both owned handles is faster overall than either paired or
single-side borrowed traversal.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-007-borrowed-rw-sequence-args/rust-callgrind-borrowed-rw-sequence-args.out \
  target-wsl-280-borrowed-rw-sequence-args/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
