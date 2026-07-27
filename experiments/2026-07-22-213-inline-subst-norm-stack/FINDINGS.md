# Rejected inline substitution-normalization stack

## Question

Can `Substitution::norm_term` use eight operation-local term slots and retain
its existing vector only for overflow, approximating C's local pointer stack
without increasing `Substitution` size?

## Setup

- Parent source: commit `023e4063` (`Inline common first-order MGU jobs`),
  accepted Experiment 211. Experiment 212 records a rejected constant ablation
  and restores this same source.
- Candidate: add an eight-`Option<Term>` local LIFO, push into the retained
  `norm_stack` vector only while the inline stack is full or overflow is live,
  and always pop overflow before inline work.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-211-inline-mgu-job-deque/rust-callgrind-inline-mgu-job-deque.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-213-inline-subst-norm-stack/rust-callgrind-inline-subst-norm-stack.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The spill regression uses ten variables and verifies exact left-to-right
binding order, empty retained overflow, capacity retention, and backtracking.
The implementation uses no unsafe or uninitialized storage.

## Deterministic result

The candidate preserves the expected 4,873-processed-clause LUSK6 proof but
retires 10,677,584,490 instructions. This is 32,937,286 above the
10,644,647,204-instruction parent, a 0.309426% regression. The hypothetical
C/Rust ratio worsens from 2.025869 to 2.032137.

The candidate removes 86,986 Rust allocations; `malloc` falls by 3,914,370
instructions, `free` by 2,438,996, and the comparable raw-vector growth entry
by 2,453,688. Those savings are overwhelmed inside the normalizer itself:
`Substitution::norm_term` rises from 437,245,456 to 523,670,594 exclusive
instructions, plus 86,425,138 or 19.765817%. Safe inline-slot initialization,
tier checks, and circular bookkeeping cost substantially more than the
retained-vector operations they replace.

## Validation

- All ten focused substitution tests pass, including exact spill order and
  backtracking.
- Strict pedantic library Clippy and formatting pass for the candidate.
- The candidate reaches the expected LUSK6 proof and exits zero under
  Callgrind.
- Native and compatibility matrices were intentionally skipped after the
  deterministic rejection.
- Source is restored exactly to the accepted vector traversal.

## Decision

Reject. Safe initialized inline term storage is not a useful approximation of
C's uninitialized local pointer stack in this hot path. Retain the reusable
vector and avoid retrying inline normalization without a representation that
removes the per-call slot initialization and tier overhead. The accepted
baseline remains Experiment 211 at 10,644,647,204 instructions, or 2.025869
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-subst-norm-stack.out \
  target-wsl-213-inline-subst-norm-stack/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
