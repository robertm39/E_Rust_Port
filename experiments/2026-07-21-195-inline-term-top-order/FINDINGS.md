# Neutral term-top comparator inline hint

## Question

Will a normal Rust `#[inline]` hint remove the private term-top comparator
boundary, whose representative line profile shows about six million calls and
substantial entry/exit attribution?

## Setup

- Parent source: commit `355e30a2` (`Record rejected PD-tree terminal
  sentinel`), whose executable source is accepted Experiment 190.
- Diagnostic motivation: Experiment 193 line attribution records 35,558,175
  instructions at comparator entry and 49,781,445 at exit.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-195-inline-term-top-order/rust-callgrind-inline-term-top-order.out`.

## Candidate

The only source change was `#[inline]` on the private
`term_top_order_for_problem` function. Comparison keys, first-order type
preconditions, higher-order type identity, arity, argument identity, splay
order, and tree ownership were unchanged.

## Result

All four focused term-tree tests pass and the candidate preserves the exact
4,873-processed-clause LUSK6 proof. It retires 11,588,501,371 instructions,
only 473 above the 11,588,500,898-instruction parent: a 0.000004082%
difference within layout noise. The C/Rust ratio is unchanged at 2.205501.

The compiler retains the standalone comparator at exactly 510,401,663
exclusive instructions. `splay_term_tree` and `TermTree::insert` also reproduce
exactly at 204,236,401 and 126,526,904 instructions. The ordinary hint
therefore has no material code-generation effect on the pinned release build.

## Decision

Do not retain a semantically inert source annotation. Restore
`src/terms/termtrees.rs` exactly to the accepted implementation. Native
compatibility matrices and full repository gates were skipped because the
deterministic benchmark showed no improvement after focused correctness
coverage passed. Keep the accepted baseline at 11,588,500,898 instructions and
2.205501 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-term-top-order.out \
  target-wsl-195-inline-term-top-order/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
