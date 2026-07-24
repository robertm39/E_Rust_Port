# Experiment 292: Force-inline applied-free-variable detection

## Question

Can forcing `Term::is_applied_free_var` into its hot dereference callers remove
enough call overhead to improve the whole prover without changing
applied-variable semantics?

## Baseline

- Accepted source: Experiment 290 plus Experiment 291 findings, commit
  `3ada8e66`.
- Exact default-feature LUSK6 Callgrind: 8,800,386,737 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.674873.
- `Term::is_applied_free_var` is a standalone 98,396,012-instruction function
  called 8,945,092 times over 30 profile edges.

## Candidate

Add only a measured `#[inline(always)]` boundary to
`Term::is_applied_free_var`. The predicate, argument access, dereference
algorithms, term representation, and all callers remain unchanged.

This is distinct from the accepted forced inlining of
`deref_always_step`, `term_deref_always_if_changed`, and
`term_deref_if_changed`, and from Experiment 290's accepted forced wrapper.
Those hot callers currently remain inlined while this small classification
predicate is still outlined.

## Validation

- All 18 focused term-cell and dereference tests pass.
- Formatting and `git diff --check` pass.
- Candidate WSL and native fingerprints record exactly
  `features=["default"]`.
- The candidate reaches the exact 4,873-processed-clause LUSK6 proof and exits
  zero under Callgrind.
- Three parent and eight candidate native proof runs are byte-identical:
  378-byte stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- All 384 measured native processes prove successfully and exit zero.
- Compatibility matrices and full repository gates were skipped after the
  replicated native production rejection.

## Measurement

The candidate retires 8,650,659,536 instructions, 149,727,201 below the
8,800,386,737-instruction parent. This is a 1.701371% whole-prover reduction,
and the hypothetical Rust/C ratio improves from 1.674873 to 1.646377.

The standalone 98,396,012-instruction predicate disappears. The dominant
PD-tree cursor reproduces exactly at 1,560,083,792 instructions, while
`Substitution::norm_term` falls from 443,318,643 to 436,518,815 and
`term_deref` falls from 165,567,447 to 141,446,305. The reduction is therefore
localized to predicate call removal and changed code generation in its
callers, not a changed proof search.

The WSL binary shrinks 23,936 bytes. The native candidate also shrinks from
8,931,840 to 8,901,120 bytes, a 30,720-byte reduction.

Native production timing reverses the deterministic result. Four alternating
warmup pairs preceded three independent blocks of 64 alternating measured
pairs:

| Native metric | Block 1 | Block 2 | Block 3 | Combined 192 |
| --- | ---: | ---: | ---: | ---: |
| Wall mean | +1.254044% | -0.670834% | +0.992286% | +0.522197% |
| CPU mean | +0.823045% | -0.629164% | +0.673905% | +0.285856% |
| Paired wall mean | +1.323772% | -0.567754% | +1.088188% | +0.614735% |
| Paired CPU mean | +0.901405% | -0.539366% | +0.771545% | +0.377861% |
| Candidate wall wins | 26/64 | 37/64 | 23/64 | 86/192 |
| Candidate CPU wins/ties | 26/9 | 28/12 | 19/7 | 73/28 |

The combined final 32 pairs from each block also regress:

- wall mean: +0.148588%;
- CPU mean: +0.249813%;
- paired wall mean: +0.235867%;
- paired CPU mean: +0.333801%;
- wins: 42/96 wall and 36/96 CPU, with 15 CPU ties.

Positive percentages are regressions. Block 2's improvement does not
reproduce; blocks 1 and 3 regress independently, and the combined stable
halves remain negative for the candidate.

## Result

Reject. Forced predicate inlining substantially improves deterministic
instructions and reduces both binaries, but it does not improve production
native throughput. Three independent blocks are sufficient to close this
boundary: the combined result and combined stable halves both regress.

Restore the Experiment 290 executable source byte-for-byte. Retain the
instruction profile and all three native timing blocks as evidence that this
predicate's instruction count is not a safe proxy for native performance.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-inline-applied-free-var.out \
  target-wsl-292-inline-applied-free-var/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-290d-divide-clause-sign-partition\release\eprover.exe `
  -CandidateExe .\target\native-292-inline-applied-free-var\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-019-inline-applied-free-var\native-lusk.csv
```
