# LUSK6 Search Performance

Date: 2026-07-10

## Question

Why does Rust take tens of seconds on `LUSK6.lop` after reaching the same proof-search workload that C completes in about one second?

## Setup

- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- C reference: cached build of upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` under WSL Ubuntu 24.04.
- Rust: Windows release build from this worktree.
- Shared search arguments:

```powershell
--auto --silent --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- Opt-in phase attribution:

```powershell
cargo build --release --features instrument-perf-ctr --bin eprover
```

- Raw retained outputs: `.artifacts/experiments/2026-07-10-001-lusk6-search-performance/`.

## Findings

- C and Rust process 4,897 clauses, generate 122,867 paramodulants, backward-rewrite 259 clauses, and finish with 307 processed clauses. The theorem result and proof chain agree.
- One Rust allocation-layout variant exactly matches C's 92,833 non-redundant clauses and 518,389 rewrite steps. Another repeatable variant has 92,847 and 518,287 respectively, leaving 14 extra unprocessed clauses. This residual does not change the proof but remains a parity target.
- The Rust term-tree splay allocated a dummy `TermCell` for every operation. C uses a stack-local sentinel. Explicit left/right chain roots remove that allocation while preserving the top-down splay shape.
- Indexed top rewriting rebuilt a complete clause-id `BTreeMap` for every candidate search. A map maintained by demodulator-indexed `ClauseSet`s reduced measured forward rewriting from about 9.97 seconds to 4.00 seconds.
- HCB selection rebuilt liveness from all unprocessed children on every selected clause. Selection now scans only stable source/processed/archive parent owners; periodic cleanup retains the full snapshot.
- Saturation called cleanup after every clause, and the default wrapper eagerly built the full liveness snapshot before checking whether orphan/delete-bad gates could run. Deferring that snapshot until either gate fires reduced the clean run from 29.23 seconds to 12.33 seconds.
- C's `RWDesc` reuses one substitution stack. Reusing a per-clause Rust rewrite substitution lowered the retained clean run from 30.35 seconds to 29.23 seconds before the cleanup fix.

## Timing

| Build | Seconds |
| --- | ---: |
| C reference | 1.08 |
| Rust after initial hot-path fixes | 30.35 |
| Rust with reusable rewrite substitution | 29.23 |
| Rust with lazy cleanup liveness | 12.33 |
| Final Rust trial 1 | 11.75 |
| Final Rust trial 2 | 11.11 |
| Final Rust trial 3 | 11.07 |

The final Rust median is 11.11 seconds. This is a substantial improvement over the approximately 40-second current-port baseline observed at the start of the investigation, but it is still about 10.3 times the C reference and therefore does not satisfy the project's final performance requirement.

## Falsification Checks

- Focused term-tree, term-cell-store, clause-set, rewrite, unit-simplification, cleanup, and parent-liveness tests pass.
- The optimized release run retains `SZS status Unsatisfiable`, the same processed/generated/paramodulation/backward-rewrite counts, and the same proof-producing search sequence.
- The borrowed term-key/hash experiment did not improve runtime and was removed rather than retained as speculative complexity.
- The full all-target/all-feature suite passes 3,996 library tests, every binary target, and all three schedule integration tests. Pedantic Clippy, formatting, C-source coverage, Change Later wording, Markdown links, and manual-section regeneration checks also pass.

## Conclusion

The largest costs were Rust-only repeated allocation and whole-set reconstruction around C pointer-based hot paths, not missing inferences. Removing four such costs cuts the workload to roughly one third of its prior Rust runtime without narrowing the calculus or changing strategy behavior.

## Limits

- Rust remains roughly 10 times slower than C on this case. Incremental PDT candidate traversal, stable handle-based clause/evaluation ownership, and remaining `Rc<RefCell>` term hot paths need measured follow-up work.
- The small allocation-layout-dependent rewrite-count difference remains unresolved.
- Windows Rust and WSL C timings are useful for this regression but are not a same-OS performance certification; the native WSL benchmark suite remains authoritative for final comparability.
