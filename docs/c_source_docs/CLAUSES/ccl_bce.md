<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_bce

## Source Files

- [CLAUSES/ccl_bce.h](../../../eprover/CLAUSES/ccl_bce.h)
- [CLAUSES/ccl_bce.c](../../../eprover/CLAUSES/ccl_bce.c)

## Purpose

Implements blocked clause elimination as described in Blocked Clauses in First-Order Logic (https://doi.org/10.29007/c3wq). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `BCETaskFree(t)`
- `CCL_BCE`
- `IS_BLOCKED(n)`
- `OCC_CNT(n)`

### Globals

- None found in the source scan.

### Exported Functions

- `void EliminateBlockedClauses(ClauseSet_p set, ClauseSet_p archive, int max_occs, TB_p tmp_bank)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `make_task`: Makes the task for BCE.
- `compare_taks`: Function used to order tasks inside the task queue.
- `make_sym_map`: Performs the elimination of blocked clauses by moving them from passive to archive. Tracking a predicate symbol will be stopped after it reaches max_occs occurrences.
- `make_bce_queue`: For each literal in a clause build an object encapsulating all the candidates (and how far we are in checking them), then store it in a queue ordered by number of candidates to check.
- `split_partner_literals`: Splits the literals in partner clause into those that unify and those that do not unify with "lit".
- `check_blockedness_eq`: Check if clause all equational L-resolvents between literal described by task and b are tautologies.
- `check_blockedness_neq`: Check if clause all L-resolvents between literal described by task and b are tautologies.
- `check_candidates`: Forwards the task either to the first clause that makes it non-blocked. Otherwise, forwards it to the end of the candidates list.
- `resume_task`: Forwards to the next candidate and reinserts the task into the queue.
- `do_eliminate_clauses`: Performs actual clause elimination
- `EliminateBlockedClauses`: Performs the elimination of blocked clauses by moving them from passive to archive. Tracking a predicate symbol will be stopped after it reaches max_occs occurrences.

### Dependencies

- `"ccl_bce.h"`
- `<ccl_clausesets.h>`
- `<clb_min_heap.h>`

### Compile-Time Conditions

- `CCL_BCE`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_bce.h`, `CLAUSES/ccl_bce.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 665 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Implements blocked clause elimination as described in Blocked Clauses in First-Order Logic (https://doi.org/10.29007/c3wq). the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/clauses/bce.rs` ports the clause-level blocked-clause elimination helper over the current Rust `ClauseSet` owner, including predicate occurrence maps with C's max-occurrence cutoff behavior, per-literal BCE tasks over disjoint parent copies, minimum-remaining-candidate task scheduling, blocker resumption after archive moves, non-equational L-resolvent checks through first-order MGU plus complementary-literal closure, equational same-head L-resolvent construction through generated argument disequalities, and the C-shaped `% BCE start` / `% BCE eliminated` output wrapper.
- Supported executable `--bce` preprocessing now calls the helper for first-order prune/proof-search clause-list paths after represented SInE/relevance pruning and before initial clause documentation, watchlist loading, proof-control initialization, or saturation. It also covers represented FOF formula-origin clauses and supported first-order-shaped THF formula fragments after formula-owner CNF emits represented clauses. It moves eliminated clauses to the represented proof-state archive and writes progress through the executable stdout side channel.
- Broader formula-owner preprocessing and pointer-stable proof-state handles remain pending integration points.

### Change Later

- C stores BCE tasks, blockers, and occurrence maps through raw clause pointers, so duplicate identifiers and archive/requeue aliases are still distinct. Rust currently uses compact clause identifiers for the clause-level helper because stable clause handles are not represented across `ClauseSet` moves yet; replace this with stable handles before extending BCE to full formula/proof-state preprocessing where duplicate identifiers can be observable.
- `EliminateBlockedClauses` writes progress directly to `stdout` rather than the prover's main output stream. Rust preserves that visible behavior through the executable stdout side channel; keep this isolated if the eventual output layer stops mirroring C's global stream leakage.
- C gates BCE on `PROBLEM_FO` and comments that higher-order syntax may still contain first-order problems. Rust covers represented FOF formula-origin clauses and supported first-order-shaped THF fragments after clause lowering, but full formula-owner classification should keep this boundary explicit once the temporary formula bridge is replaced.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
