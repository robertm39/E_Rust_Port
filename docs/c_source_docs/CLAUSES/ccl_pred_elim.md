<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_pred_elim

## Source Files

- [CLAUSES/ccl_pred_elim.h](../../../eprover/CLAUSES/ccl_pred_elim.h)
- [CLAUSES/ccl_pred_elim.c](../../../eprover/CLAUSES/ccl_pred_elim.c)

## Purpose

Implements (defined) predicate elimination as described in SAT-inspired eliminations for superposition (https://ieeexplore.ieee.org/document/9617710). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `ANY_SIGN`
- `BIN(x)`
- `CAN_SCHEDULE(t)`
- `CCL_PE`
- `CCSFree(junk)`
- `EXIT_ON_DIFF(res)`
- `IN_HEAP(t)`
- `ONLY_NEG`
- `ONLY_POS`
- `TASK_BLOCKED`

### Globals

- None found in the source scan.

### Exported Functions

- `void PredicateElimination(ClauseSet_p passive, ClauseSet_p archive, const HeuristicParms_p parms, TB_p bank, TB_p tmp_bank, VarBank_p fresh_vars)`

## Implementation Notes

### Internal Functions

- `CCSRemoveCl`
- `CCSStoreCl`
- `find_fcode_except`
- `max_cardinality`
- `mk_ccs`
- `mk_task`

### Source-Level Behavior

- `dbg_print`: Print the given clause to the given output stream.
- `set_proof_object`: Set proof object according to given arguments
- `mk_task`: Make a default task cell
- `CCSStoreCl`: Store a clause in the set that tracks the number of elements.
- `CCSRemoveCl`: Remove a clause from the set that tracks the number of elements.
- `PETaskFree`: Release the memory used by task object.
- `find_fcode_except`: Is there a predicate litaral with the code fc in eqn list cl (ignoring) its literal exc
- `term_vars_from_set`: Are all variables in t from the set vars?
- `unique_distinct_vars`: Is the term of the form p(X1, ..., Xn) where all X1...Xn are distinct free variables. varset is not freed, only (possibly) modified
- `potential_gate`: Is the clause of the chape (~)p(X1, ..., Xn) \/ C where all Xi are different and variables in C are subset of X1, ..., Xn
- `update_statistics`: Update number of literals, clauses and \mu measure (square of the number of different variables).
- `scan_clause_for_predicates`: Scans the clause for all the predicate literals and updates
- `max_cardinality`: What is the maximal cardinality of the set of the clauses that would be created when the symbol is eliminated?
- `cmp_tasks`: Comparator function used for ordering the tasks in the min heap. Prefers the tasks that are eligible for PE over the ones that are not, then the ones with the smallest cardinality, then the ones that have gates.
- `declare_not_gate`: Go through all the tasks and see if their potential gates are actually gates.
- `find_lit_w_head`: Find a (predicate) literal that whose head symbol is f and return it. If such a literal is not found return NULL; Extracts the other literals in rest if rest is not NULL.
- `build_neq_resolvent`: Builds regular non-equational resolvent between p_cl and n_cl clause more precisely between their literals pos and neg where pos and neg are the first positive/negative literal that contain symbol f. Undefined behavior if they do not contain f. If resolvent cannot be built (not unifiable), return NULL
- `build_eq_resolvent`: Like build_neq_resolvent() but (1) builds EQ resolvent and (2) never fails as there is no unification involved.
- `check_unsat_and_tauto`: Checks condition (4) and (5) from Definition 13. in SAT techniques paper (https://matryoshka-project.github.io/pubs/satelimsup_paper.pdf)
- `update_gate_status`: Go through all the tasks and see if their potential gates are actually gates.
- `PredicateElimination`: Preprocess the passive clause set, create corresponding predicate elimination tasks, store them in the symbol map and insert them in the task queue.
- `do_singular_elimination`: Assuming the clauses in pos_cls tree are the ones that have the positive singular occurence of sym and the neg_cls have the negative one, compute the only possible resolvent between them.
- `do_gates_against_offending`: Fixpoint computation in which all occurrences of sym in offending clauses are removed one by one by using the clauses in gates.
- `try_gate_elimination`: Fills cls with all the following resolvents: positive gates against singular negative clauses, negative gates against negative singular clauses and gates against all clauses in which symbol occurs multiple times.
- `try_singular_elimination`: Tries to eliminate the symbol described by task by performing classical singular predicate elimination.
- `react_clause_added`: Update data structures to reflect adding of the clause.
- `react_clause_removed`: Update data structures to reflect removing of a clause.
- `remove_clauses_from_state`: After symbol has successfully been eliminated, remove all clauses in which symbol appeared. Then check if this elimination makes elimination of some other symbol possible
- `measure_decreases`: Check if replacing the symbol decreases the measure.
- `eliminate_predicates`: Driver that does actual predicate elimination.

### Dependencies

- `"ccl_pred_elim.h"`
- `<ccl_clausesets.h>`
- `<ccl_satinterface.h>`
- `<che_hcb.h>`
- `<clb_min_heap.h>`

### Compile-Time Conditions

- `CCL_PE`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_pred_elim.h`, `CLAUSES/ccl_pred_elim.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1529 lines, 6 scanned public declarations, 6 scanned internal function definitions, and 31 structured function-comment blocks.
- Implements (defined) predicate elimination as described in SAT-inspired eliminations for superposition (https://ieeexplore.ieee.org/document/9617710). the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/clauses/pred_elim.rs` ports the clause-level singular predicate-elimination helper with gate recognition disabled. The helper builds C-shaped predicate tasks, honors permanent max-occurrence and conjecture-symbol blocking, requires no offending clauses for singular scheduling, builds first-order non-equational resolvents through disjoint positive-parent copies and MGU, switches globally to the C equality-aware resolver when any equality literal occurs in the passive set, generates argument disequalities for non-pattern pivots, uses the distinct-variable pivot shortcut with instantiated residual literals, filters generated tautologies, applies the C literal/clause/mu decrease test, moves eliminated source clauses to the archive, normalizes generated-clause variables before reinsertion, records `DCPEResolve` derivations, and exposes the C-shaped `% PE start` / `% PE eliminated` output wrapper. Supported first-order prune/proof-search `--pred-elim` preprocessing is wired after BCE and before goal-definition transformation when gate recognition is disabled, including supported first-order-shaped THF formula fragments after the temporary formula bridge lowers them to represented clauses.
- Gate recognition, PicoSAT-backed gate validation/core extraction, and offending-clause gate elimination remain pending. The executable adapter rejects `--pred-elim-recognize-gates=true` instead of silently approximating the C gate branch.

### Change-Later Observations

- `PredicateElimination` writes progress directly to `stdout` rather than through the prover output abstraction, just like BCE. Rust preserves this through an explicit output wrapper/side-channel; after drop-in compatibility is secured, consider routing preprocessing diagnostics through one output owner.
- C chooses the equality resolver globally when any equality literal appears in the passive set, even for predicate symbols whose own task clauses are non-equational. Rust preserves this compatibility-sensitive behavior; a later cleaned design could evaluate whether per-task resolver choice is valid under the paper's conditions and reference tests.
- `scan_clause_for_predicates` permanently blocks a task after a max-occurrence or conjecture-symbol hit by setting `size = TASK_BLOCKED`. That can leave a symbol blocked even if later eliminations remove the triggering clauses. Preserve this for now, but document any future attempt to make blocking dynamic because it can change preprocessing strength and proof shape.
- The C preprocessing driver calls predicate elimination after BCE and before goal-definition transformation, and does not add the PE removal count to the clause-preprocessing statistic returned by `ProofStatePreprocess`. Rust preserves that executable ordering and statistic boundary for the supported gate-disabled adapter.
- Gate recognition stores potential gates in both the singular sets and the gate sets, then later mutates task membership after PicoSAT validation. This aliasing is easy to misread and is tightly coupled to raw pointer set identity; a future Rust design should keep the state transition explicit if gate support is ported.
- The C task heap and cheap clause sets are raw clause-pointer based, so duplicate identifiers, archive moves, and generated/requeued clauses remain distinct by address. Current Rust clause-level work uses compact ids and rebuilds task state around `ClauseSet` ownership; stable clause handles should replace ids before full proof-state integration.
- `build_neq_resolvent` and the distinct-variable branch of `build_eq_resolvent` rely on a disjoint copy of the positive parent, a same-bank copy of the negative parent, temporary variable bindings during MGU, and a later `ClauseNormalizeVars` plus `TBInsertOpt(DEREF_NEVER)` remap before insertion. This should remain visible in Rust because term-bank sharing and variable identity are both semantic and performance constraints.
<!-- END MANUAL REVIEW: c_source_docs -->
