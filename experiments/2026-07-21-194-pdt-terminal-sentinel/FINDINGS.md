# Rejected PD-tree terminal-position sentinel

## Question

Can the existing terminal-position field use `usize::MAX` to distinguish a
nonterminal frame from an exhausted terminal frame, avoiding a
`query_stack.is_empty()` read on every cursor iteration without growing the
frame?

## Setup

- Parent source: commit `4c14c915` (`Record PD-tree line attribution`), whose
  executable source is accepted Experiment 190.
- Diagnostic motivation: Experiment 193 assigns 40,497,972 instructions to the
  loop-level query-stack emptiness check.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-194-pdt-terminal-sentinel/rust-callgrind-pdt-terminal-sentinel.out`.

## Candidate

The candidate reserved `usize::MAX` in `PdtTraversalFrame::terminal_position`
for nonterminal frames. Zero continued to mean an exhausted terminal frame,
and positive values continued to index remaining terminal entries. Initial and
new nonterminal frames received the sentinel, while completed query paths
received the terminal-entry length. The frame remained 40 bytes; traversal,
bindings, query ownership, and rollback were unchanged.

## Result

The candidate passed all 41 focused PD-tree tests and preserved the exact
4,873-processed-clause LUSK6 proof. It retired 11,656,661,711 instructions,
68,160,813 above the 11,588,500,898-instruction parent. That is a 0.588176%
whole-prover regression and raises the deterministic C/Rust ratio from
2.205501 to 2.218474.

The regression is local and unambiguous. The cursor rises from 1,488,399,423
to 1,556,558,587 exclusive instructions, adding 68,159,164 or 4.579360%.
`pop_subst_cursor_frame` reproduces exactly at 279,148,494 instructions, and
the whole-program increase differs from the cursor increase by only 1,649
instructions. Reading and maintaining the frame sentinel costs more than the
vector-length emptiness check it replaces.

## Decision

Reject the terminal sentinel and restore `src/clauses/pdtrees.rs` exactly to
the accepted implementation. Native compatibility matrices and full
repository gates were skipped because the deterministic benchmark rejected
the candidate after focused correctness coverage passed. Keep the accepted
baseline at 11,588,500,898 instructions and 2.205501 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-terminal-sentinel.out \
  target-wsl-194-pdt-terminal-sentinel/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
