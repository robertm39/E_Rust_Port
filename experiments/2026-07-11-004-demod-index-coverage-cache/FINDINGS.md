# Demodulator Index Coverage Cache

## Question

Can the remaining `LUSK6.lop` rewrite cost be reduced by matching C's assumption
that a demodulator-indexed clause set keeps all demodulators in its `PDTree`?

## Setup

- Exact baseline commit: `2c5e76a2` (`Clarify WSL sandbox visibility`).
- Workload: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments: `--auto --silent --cpu-limit=60 --memory-limit=2048
  --detsort-rw --detsort-new`.
- Native Linux C/Rust benchmark report before the candidate:
  `.artifacts/e-compare/20260711-181535-965817-benchmark/`.
- Three-run native Linux candidate report:
  `.artifacts/e-compare/20260711-183506-388648-benchmark/`.
- Callgrind output:
  `/home/rober/.cache/e-rust-port/callgrind-lusk6-2c5e76a2.out` in WSL.

The current head was built through the standard one-case benchmark harness:

```powershell
.\e-interop.ps1 benchmark -Runs 3 `
    -Corpus .\.artifacts\tmp-lusk6-corpus `
    -TimeoutSeconds 60 -MemoryLimitMb 2048
```

The exact baseline was exported with `git archive 2c5e76a2`, populated with a
separate read-only archive of the nested `eprover/` checkout, and built into an
isolated WSL target directory. Baseline and candidate were then alternated for
three `/usr/bin/time` runs each.

## Profile

The completed Callgrind proof collected 36,548,227,453 instruction references.
The largest relevant flat costs were:

| Function or group | Instruction share |
| --- | ---: |
| `rewrite_with_clause_set_plain_with_subst` | 10.54% |
| `demod_index_search_may_have_match` | 7.69% |
| PDT matching-occurrence traversal | 4.07% |
| PDT query construction | 3.22% |
| `malloc`/`free` internals shown separately | over 12% |

`demod_index_search_may_have_match` called
`demod_index_covers_demodulators`, which scanned every clause in the set. A
single root rewrite called that coverage check before search, again while
choosing indexed search, and again for every candidate request. C does not make
these scans: indexed insertion and extraction are a caller-maintained invariant.

## Retained Change

`ClauseSet` now caches the result of its conservative coverage verification.
Plain insertion, extraction, initialization, and every API that yields a mutable
clause reference invalidate the cache. The next search verifies coverage once;
subsequent searches and candidate requests use the cached result. Deliberately
unindexed transitional sets still fall back to set-order scanning.

| Build | CPU samples (s) | Median CPU | Wall median |
| --- | --- | ---: | ---: |
| Exact baseline | 4.72, 4.72, 4.82 | 4.72 s | 4.68 s |
| Coverage cache | 4.18, 4.05, 4.24 | 4.18 s | 4.04 s |

The paired CPU median improves by 11.4%. All six runs proved the theorem and
maximum RSS stayed approximately 287 MB. The independent three-run harness
measured C at 1.108 seconds and Rust at 4.101 seconds, a remaining 3.703x ratio.

## Initial Query-State Follow-Up

A follow-up stored the already-built PDT query as one vector of query cells
instead of copying it into four parallel vectors. All 28 focused PDT tests
passed, but an extended seven-pair sample did not establish an improvement:

| Build | Median CPU | Median wall |
| --- | ---: | ---: |
| Coverage cache only | 4.59 s | 4.45 s |
| Coverage cache plus single query vector | 4.58 s | 4.47 s |

The refactor was removed at that point because fewer allocations alone were not
enough evidence on this allocator-sensitive workload. A later post-cache profile
retested the same representation with ten alternating pairs and an independent
`GEO288+1.p` falsification case. That broader run measured a 3.1% LUSK6 median
CPU improvement and a small GEO improvement, so the single-vector state was
retained later. See
[`../2026-07-11-006-post-cache-callgrind/FINDINGS.md`](../2026-07-11-006-post-cache-callgrind/FINDINGS.md).

## Falsification Checks

- The indexed and fallback search tests pass, including cache invalidation after
  mutable clause access.
- All paired runs report `SZS status Unsatisfiable`.
- The native harness reports matching C/Rust behavior for the one-case corpus.
- The initially rejected query-state result is preserved here; the later
  re-evaluation records the evidence used to retain it.
- The nested upstream checkout remains clean.

## Conclusion

Repeated whole-set coverage validation was a Rust-only hot-path cost. C relies
on an implicit indexed-set lifecycle contract; Rust retains a checked fallback
but amortizes verification across unchanged set state. The remaining native
performance gap is still far above the required 1.10 ratio and remains active.
The subsequent full five-run benchmark at
`.artifacts/e-compare/20260711-200230-752173-benchmark/` measured a 3.8238x
aggregate ratio; `LUSK6.lop` measured 3.5069x with matching behavior.

## Limits

- The retained cache does not make the index lifecycle type-safe; it invalidates
  conservatively around existing mutable APIs.
- Full `PDTreeFindNextDemodulator` cursor/substitution coupling remains unported.
- The exact before/after attribution covers one rewrite-heavy problem. The full
  benchmark validates the retained candidate across the standard corpus but
  does not isolate this cache's effect on each other case.
