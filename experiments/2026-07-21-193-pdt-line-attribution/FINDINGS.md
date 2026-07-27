# PD-tree line attribution

## Question

Which safe Rust operations dominate the accepted first-order PD-tree cursor,
and can optimized line tables provide representative attribution without
materially changing the deterministic workload?

## Setup

- Source: commit `01fb6fe6` (`Record rejected PD-tree bulk restoration`), whose
  executable source is accepted Experiment 190.
- Build: the ordinary release profile with `CARGO_PROFILE_RELEASE_DEBUG=1`;
  this adds line tables without changing source or optimization level.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Accepted profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Line-table profile:
  `.artifacts/experiments/2026-07-21-193-pdt-line-attribution/rust-callgrind-pdt-line-attribution.out`.

## Representativeness

The line-table binary preserves the exact 4,873-processed-clause proof and
retires 11,588,505,462 instructions. This is only 4,564 instructions or
0.000039% above the accepted 11,588,500,898-instruction profile, so its
line-level distribution is representative of the pinned optimized binary.

## Cursor attribution

The accepted `search_next_matching_occurrence_impl` costs 1,488,399,423
exclusive instructions over 880,523 calls. Line tables assign 457,320,395 to
`pdtrees.rs`, 172,298,315 to inlined `Vec` operations, 143,957,133 to slice
indexing, 98,766,434 to `Option`, 88,367,609 to raw-vector operations,
79,659,613 to pointer operations, 75,773,092 to unsigned-integer operations,
68,118,843 to cell borrowing, and 37,369,947 to `IntMap` lookup.

The largest directly attributed cursor checks are:

- `query_stack.is_empty()`: 40,497,972 instructions;
- frame `next_step >= 2`: 40,276,628 instructions;
- traversal-order selection: 30,235,126 instructions;
- traversal-step dispatch: 30,235,126 instructions;
- loading the variable-child link: 29,475,486 instructions;
- loading the current node index: 20,248,986 instructions;
- loading the current step index: 20,138,314 instructions.

`pop_subst_cursor_frame` remains 279,148,494 instructions over 5,027,453
calls. Its source-level entry and exit/drop regions account for 35,192,171 and
40,219,624 instructions, while most remaining work is attributed to inlined
`Vec`, raw-vector, and pointer operations. Experiment 192 already established
that replacing repeated pops with `truncate` makes this path 8.69% slower.

## Decision

Keep executable source unchanged. Use the profile to test a same-size frame
sentinel that distinguishes nonterminal frames from exhausted terminal frames,
allowing the cursor to consult its frame state instead of reading
`query_stack.is_empty()` on every loop iteration. Treat that as a separate
candidate because whole-program layout remains the acceptance criterion.

## Reproduction

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release --bin eprover \
  --target-dir target-wsl-193-pdt-line-attribution
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-line-attribution.out \
  target-wsl-193-pdt-line-attribution/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
