# Rejected PD-tree terminal substitution reuse

## Question

Can the first-order PD-tree cursor retain its live caller substitution while
yielding additional occurrences from the same terminal node, matching C's
`store_stack` lifecycle instead of removing and rebuilding every cursor
binding between those occurrences?

## Setup

- Parent source: commit `3277473d` (`Record rejected forced dereference
  wrapper inline`), whose executable source remains accepted Experiment 214.
- Candidate: preserve cursor-owned bindings across consecutive entries in one
  terminal node, remove any additional bindings installed by the caller, and
  backtrack to the search base before traversal leaves the terminal.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-222-pdt-terminal-subst-reuse/rust-callgrind-pdt-terminal-subst-reuse.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at
  5,254,361,329 instructions.

## Source comparison

C `PDTreeFindNextDemodulator` retains the indexed leaf substitution while its
`store_stack` traverses every `ClausePos` in that leaf. It calls
`PDTreeFindNextIndexedLeaf`, which may backtrack, only after that entry
iterator is exhausted.

Rust instead backtracks the caller substitution to its pre-search position at
the start of every cursor call and recreates the cursor bindings immediately
before returning an occurrence. The candidate tested the C lifecycle while
preserving a necessary Rust boundary: a rejected demodulator may add minimum
RHS bindings, so caller-added bindings were still removed before the next
same-leaf result.

A focused regression inserted two occurrences at one variable terminal,
added a caller-owned binding after the first result, and verified that the
second result retained only the indexed binding before final backtracking.

## Result

The candidate reaches the expected 4,873-processed-clause LUSK6 proof but
retires 10,644,748,403 instructions. This is 12,107,018 above the
10,632,641,385-instruction parent, a 0.113867% whole-prover regression. The
hypothetical C/Rust ratio worsens from 2.023584 to 2.025888.

The attempted reuse is too rare to repay its state checks:

| Metric | Parent | Candidate | Change |
| --- | ---: | ---: | ---: |
| Whole prover | 10,632,641,385 | 10,644,748,403 | +12,107,018 (+0.113867%) |
| PD-tree cursor exclusive | 1,697,827,541 | 1,708,345,061 | +10,517,520 (+0.619469%) |
| Cursor `add_binding` calls | 284,354 | 281,186 | -3,168 (-1.114104%) |
| Cursor `add_binding` instructions | 19,270,874 | 19,118,810 | -152,064 (-0.789087%) |
| Cursor `backtrack_single` calls | 15,371 | 12,203 | -3,168 (-20.610240%) |
| Cursor `backtrack_single` instructions | 445,759 | 353,887 | -91,872 (-20.610240%) |

The Rust allocation count remains exactly 6,312,342. Normalization reproduces
exactly at 437,245,456 instructions. `TermTree::insert` rises by 447,540
instructions, while the intended cursor accounts for 86.87% of the total
regression. Same-leaf reuse therefore removes real work, but only 3,168
binding pairs in this workload; the extra branch and cursor-state inspection
run on all 783,453 production cursor calls.

## Validation

- All 42 focused PD-tree tests pass with the candidate, including the new
  same-terminal caller-binding regression.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate produces the exact LUSK6 proof and exits zero under
  Callgrind.
- Source is restored byte-for-byte; all 41 retained PD-tree tests and
  formatting pass after restoration.
- Native and compatibility matrices were skipped after the deterministic
  performance gate failed.
- The vendored C checkout was not modified.

## Decision

Reject same-terminal substitution reuse and retain unconditional base
backtracking. The C-shaped lifecycle is semantically sound, but repeated
terminal bindings are too uncommon to compensate for checking that lifecycle
on every Rust cursor request. Keep accepted Experiment 214 as the baseline at
10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-terminal-subst-reuse.out \
  target-wsl-222-pdt-terminal-subst-reuse/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
