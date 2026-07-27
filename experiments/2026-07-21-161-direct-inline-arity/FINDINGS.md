# Direct inline-arity access and main-matrix audit

## Question

Does reading the `TermArgs` enum discriminant directly in `Term::arity`
recover meaningful comparator work after inline argument storage, and do the
cumulative accepted changes now pass the maintained 50-case matrix?

## Setup

- Parent source: commit `a5cc346f` (`Inline common term arguments`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,888,451,124 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-161-direct-inline-arity/rust-callgrind-direct-arity.out`.
- Project-wide compatibility gate: the maintained 50-case native-Windows Rust
  versus archived-WSL-C matrix.

## Candidate and deterministic result

The parent obtains arity by mapping `TermArgs` to a slice and reading the slice
length. The candidate added a direct enum `len` method: zero, one, and two are
returned from the discriminant, while the heap variant reads its boxed-slice
length. Layout, allocation, search semantics, and all argument access remain
unchanged.

The candidate preserves the exact proof at 12,887,833,776 instructions, only
617,348 below the parent (-0.0048%). `term_top_compare_for_problem` accounts
for 541,250 of the reduction, falling from 543,968,198 to 543,426,948, so the
nominal result is localized but operationally negligible.

## Full-matrix evidence

The accepted parent matrix at
`.artifacts/e-compare/20260721-013705-337029/` demonstrates that the cumulative
performance work has closed the former synthetic cutoff: one-second LUSK6 now
returns the exact proof in 0.943 seconds, versus C at 0.383 seconds. BOO020 and
SWV851 both return normalized `ResourceOut`. The report has one unexpected
row plus the declared sledgehammer proof difference: LCL365 chooses an
alternate valid proof while preserving status and exit code. A focused LCL
control at `.artifacts/e-compare/20260721-014708-431894/` is exact, confirming
the previously observed intermittent proof-order behavior.

The direct-arity candidate matrix at
`.artifacts/e-compare/20260721-015741-141152/` also keeps one-second LUSK6
exact, and LCL365 is exact in that run. However, BOO020 reaches an allocator
failure with exit 9 at 58.64 seconds instead of C-compatible `ResourceOut`.
That is an unexpected resource-boundary regression even though the candidate
does not change object sizes or allocation sites: its small code-generation
shift advances late BOO search into the existing 2-GiB edge.

## Decision

Reject direct inline-arity access and restore the parent source exactly. A
0.0048% deterministic improvement is not worth reopening BOO allocator
failure. Retain the parent full matrix as evidence that the one-second LUSK
acceptance owner is closed; the remaining project-wide mismatch is the
intermittent LCL proof-order difference, not status, resource handling, or the
former cutoff.
