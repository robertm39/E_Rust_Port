# Accepted owned `TermTree` insertion link moves

## Question

Can insertion into the splayed term tree transfer the old root's child link
directly, matching the C implementation's pointer moves, instead of cloning
the child and root `Rc`s and then clearing the old child link?

## Setup

- Parent source: commit `023e4063` (`Inline common first-order MGU jobs`),
  accepted Experiment 211. Experiments 212 and 213 changed only evidence after
  restoring their rejected candidates.
- Candidate: on a distinct-key insertion, use `take_left_son()` or
  `take_right_son()` to move the old root's displaced subtree into the new
  root, then move the old root into the opposite child link. The equal-key
  path remains unchanged because it must retain and return the old root.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-211-inline-mgu-job-deque/rust-callgrind-inline-mgu-job-deque.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The move is local to the post-splay insertion branches. It preserves the same
tree topology while avoiding temporary strong-reference increments and the
subsequent decrement when the old child link is cleared.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,632,641,385 instructions. This is 12,005,819 below the
10,644,647,204-instruction parent, a 0.112787% reduction. The C/Rust ratio
improves from 2.025869 to 2.023584.

`TermTree::insert` falls from 670,392,716 to 658,651,917 exclusive
instructions, a reduction of 11,740,799 or 1.751332%. That one intended hot
function explains 97.792570% of the whole-program improvement. The two link
setters also fall by a combined 50,800 instructions. This attribution confirms
that the gain comes from removing the clone/clear ownership sequence rather
than unrelated layout movement.

## Native result

The production-feature parent and candidate binaries completed 64 alternating
native Windows pairs. All 128 processes exited zero. Across the complete
sample, candidate wall mean is 2.024810 seconds versus 2.025016 for the parent,
an improvement of 0.010141%; wall median improves 0.612306%, from 2.008482 to
1.996184 seconds. Mean paired wall time regresses 0.046835%, paired median
improves 0.878926%, and the candidate wins 36 of 64 pairs.

The first candidate wall observation is a 3.566848-second scheduler outlier
whose process CPU time is only 1.937500 seconds. Process CPU across all pairs
more clearly favors the candidate: mean falls from 1.979004 to 1.960205
seconds, improving 0.949914%; median improves 0.793651%, mean paired CPU
improves 0.749410%, paired median improves 0.813062%, and the candidate wins 33
pairs with one tie.

The stable last 32 pairs agree with the CPU result. Candidate wall mean
improves 0.832535%, wall median 0.709760%, mean paired wall 0.666589%, and
paired wall median 1.607517%, with 20 wins. Candidate CPU mean improves
1.085080%, CPU median 0.793651%, mean paired CPU 0.845590%, and paired CPU
median 1.965526%, also with 20 wins and one tie. The release executable shrinks
512 bytes, from 8,645,632 to 8,645,120.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-070111-166473` has four
  cases and zero mismatches.
- Focused BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-070331-534306` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Full report `.artifacts/e-compare/20260722-070738-566128` has all 50 cases,
  zero unexpected mismatches, and only the declared `sledgehammer` output
  difference.
- All four focused term-tree tests pass.
- The full serial 4,385-test suite plus integration and binary targets passes.
- Strict all-target pedantic Clippy, formatting, the all-feature release build,
  all four documentation gates, and vendored-C cleanliness pass.

## Decision

Accept. Exact whole-program instructions improve, 97.8% of the reduction is
localized to the intended insertion function, robust native wall and CPU
statistics favor the candidate, executable size falls, and all compatibility
and repository gates pass. The accepted baseline becomes 10,632,641,385
instructions, or 2.023584 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-move-termtree-insert-links.out \
  target-wsl-214-move-termtree-insert-links/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-211-inline-mgu-job-deque\release\eprover.exe `
  -CandidateExe .\target\native-214-move-termtree-insert-links\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
