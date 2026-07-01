<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_grounding

## Source Files

- [CLAUSES/ccl_grounding.h](../../../eprover/CLAUSES/ccl_grounding.h)
- [CLAUSES/ccl_grounding.c](../../../eprover/CLAUSES/ccl_grounding.c)

## Purpose

Definitions for functions (and possibly later data types) implementing grounding of near-propositional clause sets. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GCUEncoding`
- `GroundSetCell`
- `GroundSetState`
- `GroundSet_p`
- `VarInstCell`
- `VarInst_p`
- `VarSetInstCell`
- `VarSetInst_p`

### Macros And Constants

- `CCL_GROUNDING`
- `DEFAULT_LIT_GROW`
- `DEFAULT_LIT_NO`
- `EqnLitCode(eq)`
- `GroundSetCellAlloc()`
- `GroundSetCellFree(junk)`
- `GroundSetDimacsPrintMembers(set)`
- `GroundSetLiterals(set)`
- `GroundSetMembers(set)`
- `VarSetInstCellAlloc()`
- `VarSetInstCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(GroundSetMembers(set)+(set)->non_units->empty_clauses) bool GroundSetInsert(GroundSet_p set, Clause_p clause)`
- `GroundSet_p GroundSetAlloc(TB_p bank)`
- `VarSetInst_p VarSetConstrInstAlloc(LitOccTable_p p_table, LitOccTable_p n_table, Clause_p clause, PTree_p ground_terms)`
- `VarSetInst_p VarSetInstAlloc(Clause_p clause)`
- `bool ClauseCreateGroundInstances(TB_p bank, Clause_p clause, VarSetInst_p inst, GroundSet_p groundset, bool subsume, bool resolve, bool taut_check)`
- `bool ClauseEqlitRecode(Clause_p clause)`
- `bool ClauseSetCreateConstrGroundInstances(TB_p bank, ClauseSet_p set, GroundSet_p groundset, bool subsume, bool resolve, bool taut_check, long give_up, long just_one_instance)`
- `bool ClauseSetCreateGroundInstances(TB_p bank, ClauseSet_p set, GroundSet_p groundset, bool subsume, bool resolve, bool taut_check, long give_up)`
- `bool EqnEqlitRecode(Eqn_p lit)`
- `bool GroundSetUnitSimplifyClause(GroundSet_p set, Clause_p clause, bool subsume, bool resolve)`
- `int ClauseCmpByLen(const void* clause1, const void* clause2)`
- `long ClauseSetEqlitRecode(ClauseSet_p set)`
- `long GroundSetMaxVar(GroundSet_p set)`
- `void ClausePrintDimacs(FILE* out, Clause_p clause)`
- `void ClauseSetPrintDimacs(FILE* out, ClauseSet_p set)`
- `void GroundSetFree(GroundSet_p junk)`
- `void GroundSetPrint(FILE* out, GroundSet_p set)`
- `void GroundSetPrintDimacs(FILE* out, GroundSet_p set)`
- `void PrintDimacsHeader(FILE* out, long max_lit, long members)`
- `void VarSetConstrInstFree(VarSetInst_p junk)`
- `void VarSetInstFree(VarSetInst_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `clause_get_max_lit`: Return maximal propositional literal number in clause.
- `varsetinstapply`: Given a complete VarSetInst data structure, instaniate the variables apropriately.
- `varsetinstclear`: Clear variables instatiated by a VarSetInst structure.
- `varsetinstinitialize`: Initialize a VarSetInst structure to represent the initial ground substitution (PStacks with alternatives have to be present!).
- `varinstestimate`: Return the number of clauses induced by inst.
- `varsetinstnext`: Switch to the next substitution, return false if substitution space is exhausted (true otherwise).
- `ground_set_print_unit`: Print a unit clause in standard E format.
- `ClauseCmpByLen`: Compare two clauses. The shorter one is smaller. In clauses of equal lenght, the one with more positive literals is smaller (because I say so!). Otherwise, they are considered to be in the same equivalence class.
- `EqnEqlitRecode`: Recode an equational literal as a non-equational one using $eq(l,r)=T. Return true if recoded, false otherwise.
- `ClauseEqlitRecode`: Recode a potential equational clause to a non-equational one. Return true if conversion took place.
- `ClauseSetEqlitRecode`: Recode all clauses in set, return number of conversions.
- `VarSetInstAlloc`: Create a VarSetInst for all variables occurring in clause. Does not allocate the PStacks needed!
- `VarSetInstFree`: Free a VarSetInst. Does not free the PStacks()!
- `VarSetConstrInstAlloc`: Create a VarSetInst for all variables occurring in clause, constrained as much as possible. Does allocate the PStacks needed!
- `VarSetConstrInstFree`: Free a VarSetInst. Does free the PStacks() (and expects them to be there).
- `PrintDimacsHeader`: Print a Dimacs header with the given values.
- `ClausePrintDimacs`: Print a clause in DIMACS format.
- `ClauseSetPrintDimacs`: Print a clause set in DIMACS format
- `GroundSetAlloc`: Create a initialized GroundSet structure.
- `GroundSetFree`: Free a ground set.
- `GroundSetMaxVar`: Return the index of the largest variable in set.
- `GroundSetInsert`: Insert a (ground) clause into a GroundSet. Return false if clause is already represented as a unit clause, true otherwise
- `GroundSetPrint`: Print a gound set to out.
- `GroundSetPrintDimacs`: Print a gound set in DIMACS format to out (will not print header!).
- `GroundSetUnitSimplifyClause`: Check if clause is subsumed by a unit clause from set. If yes, return true. Otherwise, remove all units resolvable with units from set and return false.
- `ClauseCreateGroundInstances`: Create all non-tautological ground instances of clause described by inst. Return false if the empty clause has been created, true otherwise.
- `ClauseSetCreateGroundInstances`: Create all ground instances of set and put them into groundset. Return false if the empty clause has been detected, true otherwise.
- `ClauseSetCreateConstrGroundInstances`: Create ground instances of set using global instantiation constraints. Return false if the empty clause has been found, true otherwise. If just_one_instance is set, just create a single instance (mapping all variables to the most frequent symbol).

### Dependencies

- `"ccl_grounding.h"`
- `<ccl_g_lithash.h>`
- `<ccl_groundconstr.h>`
- `<ccl_propclauses.h>`
- `<cio_signals.h>`

### Compile-Time Conditions

- `CCL_GROUNDING`

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

Source files reviewed: `CLAUSES/ccl_grounding.h`, `CLAUSES/ccl_grounding.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1328 lines, 29 scanned public declarations, 0 scanned internal function definitions, and 28 structured function-comment blocks.
- Definitions for functions (and possibly later data types) implementing grounding of near-propositional clause sets. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- `ClauseSetEqlitRecode` increments its result once per clause whose literals changed, even if multiple equational literals were recoded in that clause.
- `GroundSetPrint` rebuilds each stored unit as a temporary ordinary clause, prints it without an internal newline through `ground_set_print_unit`, then appends the newline in the set loop before delegating compact non-units to `PropClauseSetPrint`. Rust now preserves that visible shape through an explicit `ClausePrint`-style LOP/TPTP/TSTP ground-set renderer.
- `ClausePrintDimacs` takes a `FILE* out`, but the non-empty literal loop writes literal integers to `stdout` and only writes the trailing `0` line ending to `out`; Rust now preserves this through explicit split-writer helpers while retaining pure string renderers for intentionally single-buffer DIMACS output.
- `ClauseSetPrintDimacs` has no separate header or sorting step; it delegates to `ClausePrintDimacs` for each clause in set iteration order, including the empty-clause two-clause workaround.
- `ClauseCreateGroundInstances` prints progress comments from the low-level instance generator and then loops only while `!TimeIsUp && !MemIsLow`. The set-level grounding functions also poll those process-global flags between clauses and mark `groundset->complete` as timeout, low-memory, or complete. Rust public grounding helpers now mirror the stop/completion behavior, expose progress output through an explicit output-aware wrapper with output-format dispatch, and keep tests/reusable internals on an injected stop callback to avoid process-global races.
- Change-later candidate: C couples grounding enumeration, final/progress output, `OutputFormat`, and resource-stop globals in the same low-level functions. Keep Rust's reusable helpers and explicit output-owner boundary unless byte-for-byte executable tests require recreating the C global coupling.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
