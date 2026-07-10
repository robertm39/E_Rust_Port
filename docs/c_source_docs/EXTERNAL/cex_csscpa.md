<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# EXTERNAL / cex_csscpa

## Source Files

- [EXTERNAL/cex_csscpa.h](../../../eprover/EXTERNAL/cex_csscpa.h)
- [EXTERNAL/cex_csscpa.c](../../../eprover/EXTERNAL/cex_csscpa.c)

## Purpose

Functions and datetype realizing the CSSCPA control component. the GNU Lesser General Public License. <1> Mon Apr 10 00:10:07 GMT 2000 New

Within the source tree, this unit belongs to `EXTERNAL`. Optional external integration helpers, including CSSCPA filtering support.

Authors noted in source headers: Stephan Schulz, Geoff Sutcliffe

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CSSCPAStateCell`
- `CSSCPAState_p`
- `ClauseStatusType`

### Macros And Constants

- `CEX_CSSCPA`
- `CSSCPAStateCellAlloc()`
- `CSSCPAStateCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `CSSCPAState_p CSSCPAStateAlloc(void)`
- `bool CSSCPAProcessClause(CSSCPAState_p state, Clause_p clause, bool accept, float weight_delta, float average_delta)`
- `void CSSCPALoop(Scanner_p in, CSSCPAState_p state)`
- `void CSSCPAStateFree(CSSCPAState_p junk)`

## Implementation Notes

### Internal Functions

- `collect_subsumed`
- `find_unit_contradiction`
- `print_csscpa_state`

### Source-Level Behavior

- `ClauseStatusString`: Return a string of the clause status
- `print_csscpa_state`: Print the clause status and state statistics given.
- `collect_subsumed`: Push all clauses in set that are subsumed by clause onto subsumed. Return weight of all these clauses.
- `find_unit_contradiction`: Given a (unit) clause and a clause set, check any of the unit clauses with opposite sign in set for unifiability. Return the first clause that unifies, otherwise return NULL.
- `CSSCPAStateAlloc`: Allocate an empty, allocated CSSCPA state.
- `CSSCPAStateFree`: Free a CSSCPAState and return associated data structures.
- `CSSCPAProcessClause`: Process a clause for CSSCPA: - If it is subsumed or tautological, delete it. - If accept is true or clause subsumes clauses with a higher combined weight than clause, remove all clauses subsume by clause and add clause to state. - Otherwise delete clause. / Returns true if clause has been accepted.
- `CSSCPALoop`: Read CSSCPA-clause commands and process them. Terminate if no input remains.

### Dependencies

- `"cex_csscpa.h"`
- `<ccl_subsumption.h>`
- `<ccl_tautologies.h>`
- `<cio_output.h>`

### Compile-Time Conditions

- `CEX_CSSCPA`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for higher-order complete matching on 2026-07-10.

Source files reviewed: `EXTERNAL/cex_csscpa.h`, `EXTERNAL/cex_csscpa.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `EXTERNAL` covering 2 source file(s), about 637 lines, 7 scanned public declarations, 3 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions and datetype realizing the CSSCPA control component. the GNU Lesser General Public License. <1> Mon Apr 10 00:10:07 GMT 2000 New
- External integration code. Treat formats, command-line behavior, and temporary files as compatibility surfaces.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/external/csscpa.rs` ports the `ClauseStatusType` discriminants/string rendering, CSSCPA state counters and three clause buckets, state-line rendering, and the core `CSSCPAProcessClause` forced/check acceptance path over the current Rust clause-set and mutable-bank complete subsumption APIs.
- The current Rust slice covers tautology rejection, mutable-bank unit/full subsumption rejection, improvement by subsumed-weight and average-weight gates, unit contradiction detection, mutable-bank subsumed-clause removal, CSSCPA source propagation, and numeric `OutputLevel`-style trace text, including C's distinction between truthy `if(OutputLevel)` traces and the `OUTPRINT(1)` unit-contradiction banner. The subsumption paths now reach complete higher-order matching; contradiction unification still uses the retained unbanked MGU path.
- `CSSCPALoop` command parsing is represented as an explicit Rust loop result that preserves numeric `output_level`, `state:`, the exact buffering-plea token sequence, `accept`/`check`, optional `from` source validation, optional `improve(weight_delta, average_delta)`, current scanner-format clause parsing, and process-clause dispatch. The standalone `CSSCPA_filter` wrapper now uses this loop.

### Change Later

- `CSSCPAStateAlloc` creates separate `terms` and `tmp_terms` term banks that share one mutable signature pointer. Rust currently creates a fresh tautology work bank from the live signature for each processed clause because `TermBank` owns its signature by value; revisit a persistent work-bank design once shared signature ownership is available.
- `collect_subsumed` stores raw clause pointers on a stack and deletes those entries later. Rust collects clause identifiers plus the owning bucket because `ClauseSet` owns clauses by value; a future pointer-stable clause handle could restore direct handle deletion if CSSCPA becomes a hot executable path.
- `CSSCPAProcessClause` mixes state transitions with `GlobalOut` printing and unconditional `fflush(GlobalOut)`. Rust returns trace text from the core state operation so output routing stays explicit; executable integration should decide exactly where to preserve C's flush points.
- `ClauseStatusType` includes `requested`, which is only used by `CSSCPALoop` state requests and never returned by `CSSCPAProcessClause`. A later typed API can keep requested-state reporting separate from clause-processing results after parser compatibility is covered.
- `CSSCPALoop` accepts any positive integer after `output_level` but mutates the global flag only for `0` and `1`; other values are silently consumed and leave the prior level in place. Separately, the wrapper can seed a negative CLI `OutputLevel`, which remains truthy for direct `if(OutputLevel)` traces but does not satisfy `OUTPRINT(1)`. Rust preserves those numeric gate behaviors, but a later user-facing CLI could report unsupported levels after compatibility tests are fixed.
- `CSSCPALoop` prints `state:` requests regardless of the current `OutputLevel`, while clause-level traces are gated. Keep this split visible when the executable wrapper moves from returned trace text to real output and flush calls.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
