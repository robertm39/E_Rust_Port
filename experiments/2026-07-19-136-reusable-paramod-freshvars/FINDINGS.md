# Reusable paramodulation fresh variables and lazy PD-tree metadata

## Question

Can main indexed paramodulation reuse C's proof-state `freshvars` owner instead
of rebuilding a shadow variable bank for every candidate, and can PD-tree
search initialization avoid eagerly materializing metadata that C computes
only along visited paths?

This experiment also checks whether Windows Job Object memory limits preserve
the requested C `RLIMIT_DATA` allowance rather than charging process overhead
against the prover's data budget.

## Setup

- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Rust branch: `codex/initial-rust-port-slice`.
- Shared proof arguments: `--auto --silent --cpu-limit=60
  --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1`.
- Deterministic profile fixture: unchanged
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`, with a 600-second CPU
  limit under WSL Callgrind.
- Production candidate: `target/pdt-lazy-metadata/release/eprover.exe`,
  SHA-256
  `E7D480434AD39D8910A829B1FEA69CB95535BAFF84E98CF734ED084D331D225F`.

The ignored proof and resource corpora are reproducibly staged with:

```powershell
& experiments\2026-07-19-136-reusable-paramod-freshvars\stage_corpora.ps1
```

The script preserves `Axioms/HEN001-0.ax` beneath the proof corpus so
`HEN011-2.p` resolves its relative TPTP include. The earlier focused report at
`.artifacts/e-compare/20260719-105812-516762/` predates that staging fix; its
GEO288 and LUSK rows are valid, but its HEN row is only matching include-error
behavior and is not proof-search evidence.

## Source comparison

C allocates `ProofStateCell::freshvars` once, pairs it with the live term-bank
variable bank, and calls `VarBankResetVCounts()` before each indexed
paramodulant constructor. Rust instead called `fresh_var_bank_for_clauses()`
inside both indexed construction directions. That copied the live clauses'
variables and then destroyed the bank for every accepted candidate.

C `PDTreeSearchInit()` initializes a `TermLRTraverse` stack over the query and
does not compute a node's token, type, or weight until traversal reaches it.
Rust's cursor needs a flat query and subtree spans, but it also stored token,
type UID, and weight in every `PrefixQueryCell` during initialization. Most
rewrite searches can reject before visiting all those cells.

## Candidate changes

The main proof-control path now passes the proof state's paired `fresh_vars`
bank through indexed paramodulation. It resets typed counters immediately
before every constructor, matching the two C constructors. Existing public
indexed wrappers retain their standalone scratch-bank behavior for callers
whose variables are not registered in a proof state's live bank.

`PrefixQueryCell` now stores only the shared term and its subtree span. Token,
type UID, and standard weight are derived on demand for a visited cursor cell.
The flat cursor and its search order are unchanged; this is an intermediate
C-lazy representation rather than a new matching algorithm.

On Windows, the Job Object limit now translates the requested data limit to a
whole-process limit by adding one eighth, capped at 256 MiB. This preserves a
small proportional allowance at low limits and covers executable, stack, and
allocator commit at the maintained 2 GiB setting. Linux continues to apply
the requested `RLIMIT_DATA` directly.

## Deterministic profile results

The retained pre-experiment LUSK6 profile contains 19,899,749,157
instructions. It attributed about 10.7% to fresh variable-bank creation,
population, and destruction.

| Candidate | Instructions | Change from prior | Relevant call-tree result |
| --- | ---: | ---: | --- |
| Retained baseline | 19,899,749,157 | - | fresh-bank ownership about 10.7% |
| Reusable `freshvars` | 18,451,265,872 | -7.28% | fresh-bank construction/destruction absent |
| Reusable bank plus compact PD query | 18,110,999,911 | -1.84% | `record_search_init` 10.81% to 6.09% |

The combined reduction is 8.99%. The compact query deliberately moves some
work into paths that are actually visited:
`search_next_matching_occurrence_with_subst` rises from 8.15% to 9.96%, while
`record_search_init` falls by about 892 million instructions. The proof,
selection counts, and call counts remain identical. Profiles are retained at:

- `.artifacts/experiments/2026-07-19-136-reusable-paramod-freshvars/callgrind-current.out`;
- `.artifacts/experiments/2026-07-19-136-reusable-paramod-freshvars/callgrind-compact-query.out`.

The next construction hotspot is now `EqnList::copy_repl` at 16.25%, almost
entirely `TermBank::insert_repl`. Rewrite normalization remains the overall
dominant owner.

## Compatibility and resource results

Reusable fresh-variable ownership changes variable chronology to the C order.
The first full report at
`.artifacts/e-compare/20260719-100515-606536/` makes GEO288 exact and reduces
its Rust wall time from the prior roughly 64 seconds to 12.31 seconds.
`LUSK6.lop` and `LUSK6ext.lop` are exact at 3.41 and 7.80 seconds. HEN011 and
the synthetic one-second LUSK6 case are still throughput misses in this
pre-query-compaction report.

The Windows mapping required falsification rather than a single successful
sample:

| Whole-process allowance at 2 GiB | BOO020 | SWV851 | Report |
| --- | --- | --- | --- |
| none | allocator exit 9 | exact `ResourceOut`/8 | `20260719-100515-606536` |
| cooperative 15/16 stop | status text but exit 9 | exact | `20260719-102530-491611` |
| +64 MiB | exact focused sample, later exit 9 | later exit 9 | `20260719-103735-820992`, `20260719-110428-194962` |
| +128 MiB | exact | allocator exit 9 at 56.55 s | `20260719-111726-401197` |
| +256 MiB cap | exact at the smaller allowance | exact `ResourceOut`/8 | `20260719-111726-401197`, `20260719-112436-524678` |

The rejected cooperative memory check rendered a generic resource message too
close to the Job boundary and could not reproduce C's hard-limit exit. It has
been removed. The final mapping instead leaves the existing CPU/resource
reporting path in control and changes only what Windows counts against the
requested data allowance.

The first compact-query full report at
`.artifacts/e-compare/20260719-110428-194962/` has exact GEO288 and
`LUSK6ext`, but it was run with the rejected +64 MiB cap and therefore records
both resource allocator failures. It is retained as falsification evidence,
not as the final compatibility result.

The final maintained report is
`.artifacts/e-compare/20260719-113621-685990/`. BOO020 and SWV851 both return
exact `ResourceOut`/8 output, and the synthetic 16 MiB case remains exact.
GEO288 proves in 11.73 seconds with exact output. HEN011 now proves with exact
output in 60.40 seconds of harness wall time, closing the maintained status
gap narrowly. `LUSK6ext` remains exact.

The report has two unexpected rows plus the declared `sledgehammer.p`
proof-text difference. The synthetic one-second LUSK6 case remains a real
`Unsatisfiable` versus `ResourceOut` outcome gap. Unlimited LUSK6 proves on
both sides but selected a longer Rust proof in this matrix: candidate clause
82 is an additional `spm(80,14)` branch, while the C clause appears later as
candidate clause 98. Three immediate focused reports at
`.artifacts/e-compare/20260719-114726-754296/`,
`.artifacts/e-compare/20260719-114743-960805/`, and
`.artifacts/e-compare/20260719-114801-592445/` are exact. Five more direct
C/Rust process pairs all selected the same C clause 82. This is retained as an
intermittent proof-order gap, not declared as an expected difference.

## Falsification checks

- The reusable API test pre-allocates typed variables, constructs an indexed
  paramodulant, verifies that the caller's counter was reset, and pins the
  exact derived clause.
- All 37 focused PD-tree tests pass with lazy metadata, including subtree
  spans, repeated-variable matching, type constraints, and traversal order.
- Callgrind uses identical LUSK6 proof and call counts, separating a real
  instruction reduction from host timing noise.
- Focused GEO288 and both LUSK proofs retain exact normalized C output.
- BOO020 and SWV851 were rerun separately at each relevant Windows allowance;
  a single early success was not accepted as sufficient evidence.
- The 16 MiB Windows mapping remains proportional, has a unit regression, and
  leaves the maintained synthetic memory-limit case exact.
- The final LUSK6 proof-text mismatch was checked with three focused harness
  runs and five direct process pairs. All eight follow-ups are exact, so the
  single full-matrix difference remains open as process-layout-sensitive proof
  order rather than being hidden or reclassified.
- The vendored C checkout is never modified.

## Decision

Accept reusable proof-state fresh-variable ownership and on-demand PD-tree
query metadata. They follow C's ownership and laziness more closely, reduce
deterministic LUSK6 instructions by 8.99% together, and preserve exact proofs.

Accept the proportional, capped Windows whole-process allowance: the final
maintained matrix confirms BOO020, SWV851, and the small synthetic memory-limit
case together. HEN011 now closes narrowly. Keep the one-second LUSK6 outcome
and intermittent unlimited LUSK6 proof order open; do not classify either as
an expected difference.
