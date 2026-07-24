# Experiment 279: Borrowed occurrence-check traversal

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can first-order occurrence checking follow variable bindings and term arguments
under scoped borrows, matching C's raw-pointer recursion without acquiring an
owned `Rc<TermCell>` handle at every visited node?

## Setup

- Parent source: commit `9fb0c740` (`perf: reject changed-only recursive
  normalization`); executable source remains accepted Experiment 270.
- Accepted compact profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: expose the existing crate-private binding borrow, follow
  free-variable chains recursively under that borrow, and traverse ordinary
  argument slots by reference. Bound applied-variable heads retain the
  existing full-dereference expansion path.
- Variant B additionally prevents the recursive occurrence-check helper from
  being duplicated into callers, testing whether code expansion rather than
  the borrowed traversal causes any whole-program reversal.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted line profile attributes 138,342,949 instructions to 320,224
first-order `occur_check` entries and another 98,899,089 instructions to
430,524 recursive entries. The accepted implementation calls general
`term_deref` and clones each recursive argument; C `OccurCheck` follows raw
binding and argument pointers.

## Results

### Borrowed traversal

The candidate proves the expected unsatisfiable result, and the intended local
owner improves substantially. In directly comparable optimized line-table
profiles, the top-level 320,224-call `occur_check` edge falls from 138,342,949
to 77,001,479 instructions, a reduction of 61,341,470 or 44.340149%.

That local gain does not transfer to the whole program. The compact candidate
rises from 8,992,812,925 to 9,018,624,021 instructions, a regression of
25,811,096 or 0.287019%. The representative line-table build similarly rises
from 8,994,036,876 to 9,018,494,043 instructions, or 0.271926%.

The all-feature native executable grows from 8,952,320 to 9,012,224 bytes,
suggesting that the more complex recursive borrow path changes inlining and
code layout well beyond the locally improved function.

### No-inline containment

Variant B keeps the borrowed traversal but applies `#[inline(never)]` to the
recursive occurrence-check entry. It also proves the expected result, but
retires 9,019,160,779 instructions: 26,347,854 or 0.292988% above the parent
and 536,758 instructions above Variant A. Preventing caller duplication does
not contain the reversal.

## Validation

- All 21 default and 22 all-feature MGU tests pass for the borrowed candidate.
- A focused regression covers a multi-link borrowed binding chain leading
  through an ordinary term argument.
- Strict all-feature library pedantic Clippy and formatting pass for Variant
  A.
- Compact and line-table WSL Callgrind runs for Variant A, plus compact
  Callgrind for Variant B, all prove LUSK6 and exit zero.
- Native timing and compatibility matrices are skipped after both exact
  instruction profiles reject the performance-only change.
- After rejection, the binding borrow visibility, occurrence-check
  implementation, and focused regression are removed and accepted sources
  are restored byte-for-byte.

## Decision

Reject both variants. Borrow-scoped occurrence checking cuts its intended
local owner by 44.340149%, but the larger recursive implementation regresses
the complete optimized binary with and without forced no-inlining. Keep
Experiment 270 as the accepted baseline at 8,992,812,925 instructions, or
1.711495 times C.

Future first-order MGU work should preserve the accepted occurrence-check
code shape; the owned queue, specialized dereference, inline job deque, and
borrowed occurrence traversal have now all been isolated.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-006-borrowed-occur-check/rust-callgrind-borrowed-occur-check.out \
  target-wsl-279-borrowed-occur-check/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
