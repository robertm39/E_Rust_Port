# Experiment 260: Defer eval-store indexing

## Status

Accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can Rust preserve C's deliberately unindexed `eval_store` lifecycle instead
of constructing an evaluation splay index that is immediately removed and
then reconstructed in `unprocessed`?

## C lifecycle

C inserts newly generated clauses into `state->eval_store` before they have
evaluations. `eval_clause_set()` then attaches evaluations in place and does
not update the set's empty evaluation roots. The following
`ClauseSetExtractFirst()` calls therefore find no source evaluation node to
remove, and `ClauseSetInsert(state->unprocessed, handle)` indexes each
evaluation exactly once in its durable owner.

The accepted Rust parent instead called `rebuild_eval_indices()` after the
in-place batch. Moving a clause to `unprocessed` consequently:

1. inserted it into temporary `eval_store` evaluation trees;
2. splayed those trees again to remove it;
3. inserted it into the durable `unprocessed` trees.

The candidate removes that rebuild. Safe evaluation-object handles also remain
absent while clauses are in the deliberately unindexed store and are assigned
by the destination insertion. Focused tests now verify that evaluated
`eval_store` clauses preserve order and weights while `find_best()` remains
empty, then verify that the move constructs the final index and selects the
same clause.

## Baseline and deterministic measurement

- Accepted source: Experiment 245.
- Accepted exact LUSK6 Callgrind: 9,898,434,766 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Accepted Rust/C ratio: 1.883851.

The default-feature candidate preserves the exact 4,873-processed-clause
LUSK6 proof and retires 9,596,668,097 instructions. That is 301,766,669 fewer
instructions, or 3.048630%, and improves the Rust/C ratio to 1.826420.

The evaluation-index traffic falls as predicted:

| Exclusive owner | Accepted | Candidate | Change |
| --- | ---: | ---: | ---: |
| `EvalIndexTree::splay` | 306,825,308 | 122,822,286 | -184,003,022 (-59.969962%) |
| `index_clause_evaluations` | 80,191,181 | 42,043,416 | -38,147,765 |
| `ClauseSet::extract_at_slot` | 71,778,854 | 44,171,490 | -27,607,364 |

Splay calls fall from 1,438,060 to 399,768, a reduction of 1,038,292 or
72.200882%. The remaining calls are the durable `unprocessed` and other
ordinary ClauseSet index operations.

The raw candidate profile is:

```text
.artifacts/experiments/2026-07-23-022-defer-eval-store-index/rust-callgrind-defer-eval-store-index.out
```

## Native production measurement

After a four-pair warmup, 64 alternating default-feature Windows pairs all
prove the theorem and exit zero. Both accepted and candidate executables are
exactly 8,654,336 bytes.

Across all 64 pairs:

- wall mean improves 3.370926%, from 1.551252 to 1.498960 seconds;
- process-CPU mean improves 2.483766%, from 1.503906 to 1.466553 seconds;
- wall and CPU medians improve 2.755830% and 2.105263%;
- mean paired wall and CPU changes improve 3.211245% and 2.388099%;
- the candidate wins 56 wall pairs and 49 CPU pairs, with four CPU ties.

The stable last 32 pairs remain positive:

- wall and CPU means improve 4.079867% and 2.735265%;
- wall and CPU medians improve 2.669507% and 3.157895%;
- mean paired wall and CPU changes improve 3.824771% and 2.643260%;
- the candidate wins 29 wall pairs and 27 CPU pairs, with one CPU tie.

The measured rows are retained in `native-lusk.csv`.

An initial diagnostic accidentally enabled all Cargo features, including
`instrument-perf-ctr`. Its 8,719,872-byte candidate was not comparable to the
accepted default-feature binary and its timings were discarded. The
default-feature rebuild restored exact binary-size parity before every
accepted measurement above.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260723-115857-281813` has GEO,
  HEN, LUSK6, and LUSK6ext exact with zero mismatches.
- Strict resource report `.artifacts/e-compare/20260723-120043-540053` has
  BOO020 and SWV851 exact at the maintained 60-second/2-GiB limits.
- The first loaded 50-case report
  `.artifacts/e-compare/20260723-120451-488741` observed one non-reproducible
  `lists.p` normalized-output difference plus the declared `sledgehammer`
  difference. Six immediate isolated C/Rust `lists.p` reports were exact.
- Final maintained report `.artifacts/e-compare/20260723-122830-344984`
  completes all 50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` normalized-output difference.
- The full serial all-target/all-feature suite passes 4,388 library tests plus
  every integration and binary target.
- Strict default-library and all-target/all-feature pedantic Clippy pass.
- The locked all-feature release build, formatting, `git diff --check`, all
  four C-source documentation gates, and vendored-C cleanliness pass.

## Decision

Accept. The change restores the original C set/index lifecycle, removes an
entire temporary insert/remove cycle, improves exact instructions by 3.05%,
improves native wall and CPU time by material margins, and preserves proof,
resource, and maintained-matrix behavior. The accepted baseline becomes
9,596,668,097 instructions, or 1.826420 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-defer-eval-store-index.out \
  target-wsl-260-defer-eval-store-index/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-245-single-maximal-candidate-vector\release\eprover.exe `
  -CandidateExe .\target\native-260-defer-eval-store-index\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-022-defer-eval-store-index\native-lusk.csv
```
