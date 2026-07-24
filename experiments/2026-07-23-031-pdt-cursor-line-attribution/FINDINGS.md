# Experiment 269: Refresh PD-tree cursor line attribution

## Status

Diagnostic experiment for Bead `E_Rust_Port-j76.5.3`; production source is
unchanged.

## Question

After the accepted term-bank and allocator improvements, which safe Rust
operations still dominate the first-order PD-tree matching cursor, and is
there a bounded next candidate that does not add another whole-function
monomorphization?

## Setup

- Source: commit `36c09e8f`, with Experiment 267 as the accepted production
  baseline.
- Accepted compact profile: 9,024,090,576 instructions.
- Diagnostic build: ordinary release optimization plus
  `CARGO_PROFILE_RELEASE_DEBUG=1`.
- Workload: exact LUSK6 under WSL Callgrind with `--auto --silent
  --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.
- Raw diagnostic profile:
  `.artifacts/experiments/2026-07-23-031-pdt-cursor-line-attribution/rust-callgrind-pdt-cursor-lines.out`.

## Representativeness

The line-table binary preserves the exact `Unsatisfiable` proof and retires
9,025,924,142 instructions. It is 1,833,566 instructions or 0.020319% above
the accepted profile, with a 1.717797 Rust/C ratio.

The first-order `search_next_matching_occurrence_impl::<true>` entries sum
exactly to 1,581,288,798 exclusive instructions, identical to the accepted
compact profile. The cursor attribution is therefore directly comparable.

## Attribution

Optimized debug locations split inlined cursor work across the implementation
and standard-library owners:

| Exclusive location inside the cursor | Instructions | Cursor share |
| --- | ---: | ---: |
| `src/clauses/pdtrees.rs` | 450,266,540 | 28.4747% |
| `alloc::vec` | 254,796,834 | 16.1132% |
| slice indexing | 138,042,494 | 8.7297% |
| `Option` | 102,672,049 | 6.4929% |
| `RawVec` | 88,723,210 | 5.6108% |
| pointer operations | 76,767,498 | 4.8547% |
| `RefCell` | 69,879,889 | 4.4192% |
| `usize` operations | 67,479,232 | 4.2674% |
| memory operations | 41,936,109 | 2.6520% |
| symbol `IntMap` | 37,781,724 | 2.3893% |

The current-frame state machine repeatedly indexes
`cursor.frames[frame_index]`:

- current node load: 20,248,986 instructions;
- terminal-query test: 40,497,972;
- step-index load and completion test: 20,138,314 plus 40,276,628;
- traversal-order selection and dispatch: 30,235,126 each;
- variable-link load and update: 29,475,486 plus 9,065,522.

These are not all bounds checks, but they sit on top of the 138,042,494
instructions attributed to slice indexing and the 254,796,834 attributed to
`Vec`.

The other conspicuous bounded edge is the 2,303,361-call
`advance_variable_query` helper at 69,100,975 inclusive instructions. It is
not a new candidate: Experiment 233 forced it inline, improved Callgrind, and
then regressed replicated native timing. Traversal-order monomorphization,
direct phase encoding, query-type caching, variable-scan widening, and compact
binding indices have likewise already been falsified.

## Next candidate

Test one safe representation-neutral change: obtain the active frame through
`frames.last_mut()` and retain that current-frame borrow while reading or
updating its node, terminal, traversal-step, effective-weight, and
variable-link fields. Release the borrow before pushing, popping, or calling a
helper that mutates the frame vector.

This follows C `pdtree_forward`'s single `tree_pos` cursor boundary while
retaining Rust's existing safe arena indices, frame layout, traversal order,
query vectors, binding representation, and first-order/higher-order
specialization count. It must still pass whole-program Callgrind before native
timing because optimizer layout remains sensitive.

## Decision

Keep production source unchanged in Experiment 269. Use the active-frame
borrowing candidate as the next isolated experiment; do not retry the already
rejected line-level alternatives above.

## Reproduction

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release --bin eprover \
  --target-dir target-wsl-269-pdt-lines
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-cursor-lines.out \
  target-wsl-269-pdt-lines/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
