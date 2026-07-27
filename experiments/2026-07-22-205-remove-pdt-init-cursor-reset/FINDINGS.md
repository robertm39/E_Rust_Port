# Remove redundant PD-tree initialization cursor reset

## Question

Can `PdTree::record_search_init` rely on the existing single-search lifecycle
instead of clearing a substitution cursor that construction and every valid
search exit already leave reset?

## Setup

- Parent source: commit `aec1f638` (`Record rejected PD-tree search init
  inline`), whose executable source is accepted Experiment 203.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-203-force-inline-pdt-frame-pop/rust-callgrind-force-inline-pdt-frame-pop.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-205-remove-pdt-init-cursor-reset/rust-callgrind-remove-pdt-init-cursor-reset.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The accepted profile records 877,339 searches. Experiment 149 already removed
the analogous redundant initialization-time query recycle while preserving
the exit-time owner boundary.

## Lifecycle and implementation

`PdtSubstCursor::new` constructs empty frame, binding, query-term, and
query-step vectors with `initialized == false`. Every valid search calls
`record_search_exit`, which resets those same fields. A subsequent
`record_search_init` already asserts that there is no active search and that
the previous search state was recycled, so its second cursor reset can do no
useful work.

The candidate removes only that initialization-time mutable borrow and reset.
A debug assertion now checks all four cursor vectors and the initialized flag
at the lifecycle boundary. Exit-time reset, retained vector capacity, lazy
cursor start, substitution backtracking, query ownership, and traversal order
are unchanged. The existing live-substitution cursor test now proves that exit
fully resets the cursor and that the following initialization observes the
reset state before lazy traversal begins.

Upstream C likewise permits only one active `PDTree` search and asserts that
precondition at initialization. Its stack initialization/reset belongs to the
search lifecycle; the Rust-owned cursor additionally clears owned terms at
exit so they are not retained between searches.

## Performance result

The candidate preserves the expected LUSK6 proof and retires 10,875,198,328
instructions. This is 39,479,701 below the 10,914,678,029-instruction parent,
a 0.361712% whole-prover reduction. The deterministic C/Rust ratio improves
from 2.077261 to 2.069747.

The comparable search-init plus search-exit aggregate falls from 201,643,333
to 162,163,078 instructions, saving 39,480,255 or 19.579251%. This differs
from the whole-program saving by only 554 instructions. The 1,697,827,541-
instruction matching cursor, term-tree insertion, substitution normalization,
and allocator hotspots reproduce exactly, localizing the improvement to the
changed lifecycle path.

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate wins nine pairs and the parent seven. Candidate mean is 1.940457
seconds versus 1.950150, a 0.497027% improvement; candidate median is 1.936967
versus 1.955645, a 0.955066% improvement. Mean paired improvement is
0.360832%. All 32 measured runs prove with exit zero. The executable shrinks
from 8,650,240 to 8,632,320 bytes, a 17,920-byte reduction.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260722-023638-966851/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Resource report `.artifacts/e-compare/20260722-023839-716568/` has zero
  mismatches for BOO020 and SWV851 at the 60-second, 2-GiB boundary.
- The recent clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. Subsequent
  accepted changes have focused exact proof and resource evidence.

## Validation

- All 41 focused PD-tree tests pass, including the strengthened lifecycle
  regression.
- The complete serial suite passes 4,384 library tests plus every integration
  target and feature.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept removal of the redundant initialization-time cursor reset. The debug
invariant and test pin the exit-owned lifecycle, deterministic and native
performance improve, the executable shrinks, and focused proof/resource
compatibility is exact. Keep the main parity issue open at 2.069747 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-remove-pdt-init-cursor-reset.out \
  target-wsl-205-remove-pdt-init-cursor-reset/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
