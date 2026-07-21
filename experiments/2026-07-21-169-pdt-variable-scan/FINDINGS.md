# Rejected PD-tree inner variable scan

## Question

Can the PD-tree cursor scan consecutive rejected variable alternatives inside
one variable step, matching C `pdtree_forward`, while caching the current query
term's type, weight, and identity for that scan?

## Setup

- Parent source: commit `ea47d0cc` (`Streamline first-order PD-tree symbol
  lookup`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,557,467,650 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-169-pdt-variable-scan/rust-callgrind-variable-scan.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The parent cursor consumes one variable-child link per outer state-machine
iteration. The candidate added an inner scan that advanced through type,
binding, and constraint failures until it found a traversable child or
exhausted the list. It read the query type UID, standard weight, and term
identity once per scan, using identity integers for the same pointer-equality
checks already expressed by `Term::eq`. Child order, frame state, speculative
bindings, weight adjustment, and constraint timing remained unchanged. All 39
focused PD-tree tests passed with the exact LUSK6 proof.

## Result

The candidate retires 12,665,478,810 instructions, 108,011,160 above the
parent (+0.8601%). The regression is localized to the intended cursor:
`search_next_matching_occurrence_impl` rises from 1,582,253,858 to
1,689,973,180 exclusive instructions, an increase of 107,719,322 (+6.808%).
The larger loop and eager query-metadata reads make the common successful or
short alternative path more expensive than the saved outer-loop redispatch.

Because the deterministic performance gate failed, native proof/resource and
full-matrix runs were intentionally skipped. The source was restored exactly
to `ea47d0cc`.

## Decision

Reject the inner variable-alternative scan and keep one child attempt per
compact outer cursor step. C benefits from raw in-node pointers and direct term
fields; reproducing its loop boundary around Rust's safe arena and metadata
access is not itself an optimization. Future variable-branch work should use
measured alternative distributions before widening this hot loop.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-variable-scan.out \
  target-wsl-169-pdt-variable-scan/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
