<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_satinterface

## Source Files

- [CLAUSES/ccl_satinterface.h](../../../eprover/CLAUSES/ccl_satinterface.h)
- [CLAUSES/ccl_satinterface.c](../../../eprover/CLAUSES/ccl_satinterface.c)

## Purpose

Datatypes and declarations for efficient conversion of the proof state to propositional clauses and submission to a SAT solver. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GroundingStrategy`
- `SatClauseCell`
- `SatClauseFilter`
- `SatClauseSetCell`
- `SatClauseSet_p`
- `SatClause_p`
- `SatSolver_p`

### Macros And Constants

- `CCL_SATINTERFACE`
- `PICOSAT_BUFSIZE`
- `SatClauseCellAlloc()`
- `SatClauseCellFree(junk)`
- `SatClauseSetCardinality(satset)`
- `SatClauseSetCellAlloc()`
- `SatClauseSetCellFree(junk)`
- `SatClauseSetCoreSize(satset)`
- `SatClauseSetLimitReached(s)`
- `SatClauseSetMaxClausesSet(set, l)`
- `SatClauseSetNonPureCardinality(satset)`

### Globals

- `extern char* GroundingStratNames[]`

### Exported Functions

- `(PStackGetSP((s)->set))) SatClauseSet_p SatClauseSetAlloc(void)`
- `ProverResult SatClauseSetCheckUnsat(SatClauseSet_p satset, Clause_p *empty, SatSolver_p solver, int sat_check_decision_level)`
- `SatClause_p SatClauseAlloc(int lit_no)`
- `SatClause_p SatClauseCreateAndStore(Clause_p clause, SatClauseSet_p set)`
- `Subst_p SubstGroundFreqBased(TB_p terms, ClauseSet_p clauses, FunConstCmpFunType is_better, bool norm_const)`
- `Subst_p SubstGroundVarBankFirstConst(TB_p terms, bool norm_const)`
- `Subst_p SubstPseudoGroundVarBank(VarBank_p vars)`
- `bool SatClauseSetCheckAndGetCore(SatClauseSet_p satset, SatSolver_p solver, PStack_p unsat_core)`
- `long SatClauseSetImportClauseSet(SatClauseSet_p satset, ClauseSet_p set)`
- `long SatClauseSetImportProofState(SatClauseSet_p satset, ProofState_p state, GroundingStrategy strat, bool norm_const)`
- `long SatClauseSetMarkPure(SatClauseSet_p satset)`
- `void SatClauseFree(SatClause_p junk)`
- `void SatClausePrint(FILE* out, SatClause_p satclause)`
- `void SatClauseSetExportToSolver(SatSolver_p solver, SatClauseSet_p set)`
- `void SatClauseSetExportToSolverNonPure(SatSolver_p solver, SatClauseSet_p set)`
- `void SatClauseSetFree(SatClauseSet_p junk)`
- `void SatClauseSetPrint(FILE* out, SatClauseSet_p set)`

## Implementation Notes

### Internal Functions

- `litstate_add_satclause`
- `litstate_check_pure`

### Source-Level Behavior

- `sat_translate_literal`: Translate a full E literal into a propositional literal.
- `litstate_add_satclause`: Add the literals of a clause to the literal state array (bit 0 indicates presence of positive instances of the atom, bit 1 represents pesence of negative instances).
- `litstate_check_pure`: Given a SatClause and a literal state array, check if any of the literals in the clause is pure.
- `prefer_conj_min_max_freq`: Prefer conjecture symbols, among those the ones rarest in conjectures, and among those the ones most frequent overall,
- `prefer_conj_max_max_freq`: Prefer symbols based on lexicographic comparision of conjecture count, total count.
- `prefer_conj_min_min_freq`: Prefer conjecture symbols, among those rare conjecture symbols, and among those overall rare symbols.
- `prefer_conj_max_min_freq`: Prefer symbols based on lexicographic comparision of conjecture count, -total count.
- `prefer_global_max_freq`: Prefer most frequent symbol.
- `prefer_global_min_freq`: Prefer least frequent symbol.
- `sat_clause_not_pure`: Does the SAT clause have no pure literals?
- `export_to_solver`: Adds the clauses that satisfy filter to the solver state. filter can be NULL in which case all the clauses are added.
- `SatClauseAlloc`: Allocate an empty, unlinked propositional clause with space for a given number of literals. Allocates space for lit_no+1 literals, where the last literal is 0 (to support efficient integration with PicoSAT). Note that other literals are not initialized (not even to 0).
- `SatClauseFree`: Free the SatClause.
- `SatClauseSetAlloc`: Allocate a SatClauseSet. This is much less flexible than full clause sets (clauses can only be added), and also carries some admin information for the translation from normal clauses to propositional clauses.
- `SatClauseSetFree`: Free a SatClauseSet (including the SatClauses).
- `SatClauseCreateAndStore`: Encode the instantiated clause as a SatClause, store it in set, and return it.
- `SatClausePrint`: Print a sat clause in DIMACS format.
- `SatClauseSetPrint`: Print a SatClauseSet.
- `SatClauseSetExportToSolver`: Exports all clauses to solver.
- `SatClauseSetImportClauseSet`: Import all (instanciated) clauses from set into satset. Return number of clauses.
- `SubstPseudoGroundVarBank`: Create a substitution binding all variables of a given sort to the smallest (first) variable of that sort (to be interpreted as an anonymous constant - this can be seen as a complete (pseudo-)grounding of all terms, literals, and clauses using this variable bank.
- `SubstGroundVarBankFirstConst`: Create a substitution binding each variable to the first constant of the proper sort.
- `SubstGroundFreqBased`: Generate a grounding substitution using occurrence-count based preference functions.
- `SatClauseSetExportToSolverNonPure`: Exports non-pure clauses to solver.
- `SatClauseSetImportProofState`: Import the all pseudo-grounded clauses in the proof state into satset.
- `SatClauseSetMarkPure`: Mark all clauses in satset that have pure literals.
- `sat_extract_core`: Extracts the original clauses pointing to the unsatisfiable core and pushes them onto core.
- `SatClauseSetCheckUnsat`: Check the satset for unsatisfiability. Return the empty clause if unsat can be shown, NULL otherwise.
- `SatClauseSetCheckAndGetCore`: Checks for unsatisfiability and extracts the unsat core in the case of unsatisfiability. If core is found true is returned.

### Dependencies

- `"ccl_satinterface.h"`
- `<ccl_proofstate.h>`
- `<cio_tempfile.h>`
- `<cte_idx_fp.h>`
- `<picosat.h>`

### Compile-Time Conditions

- `CCL_SATINTERFACE`

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

Source files reviewed: `CLAUSES/ccl_satinterface.h`, `CLAUSES/ccl_satinterface.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1249 lines, 25 scanned public declarations, 2 scanned internal function definitions, and 29 structured function-comment blocks.
- SAT bridge code; keep propositional abstraction and result interpretation aligned with PicoSAT/DPLL callers.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `SatClauseSetImportProofState` builds one pseudo-grounding substitution, imports processed positive rules, processed positive equations, processed negative units, processed non-units, and unprocessed clauses in that order, then deletes the substitution. Rust now mirrors that import order and backtracks the temporary substitution after term-bank atom encoding.
- Four conjecture-frequency tie-break helpers use assignment (`=`) where equality comparison appears intended, mutating `conj_dist_array` while comparing candidates. Rust now preserves that side effect by passing the conjecture distribution mutably through the `TBGetFreqConstTerm`-shaped callback.
- `SatClauseCreateAndStore` refuses insertions when `set_size_limit != -1` and the current cardinality is already at or above the signed limit, while `SatClauseSetLimitReached` checks exact equality with the limit. Rust now preserves that split and exposes C-shaped clause/clause-set DIMACS rendering with the trailing per-literal space before `0`.
- `SatClauseSetExportToSolver` resets the exported stack and forwards every clause to the solver in stack order; `SatClauseSetMarkPure` marks a clause as pure when any literal in that clause has only one polarity in the whole SAT clause set, and `SatClauseSetExportToSolverNonPure` then exports only clauses without such literals. Rust preserves both exported-subset shapes as safe solver-clause vectors and can now send the non-pure subset through either the internal solver or a caller-provided runtime-loaded PicoSAT solver.
- `SatClauseSetCheckUnsat` uses PicoSAT with a decision limit, resets the proof-control solver after each completed SAT check at the proof-process layer, prints the `% SatCheck found unsatisfiable ground set` comment for solver-reported UNSAT, and extracts a PicoSAT unsat core into the generated empty clause derivation. The comment is not a generic `SATCheck` success marker: proof-control normalization can find an empty clause before this helper is called and returns it without the comment or solver-result counters. C pushes extracted core parents onto a `PStack` and then pops them into the empty-clause derivation, so derivation parent order is the reverse of the extracted stack order. Rust currently uses a deterministic internal DPLL solver, deletion-minimizes the exported non-pure clause set after UNSAT, prints the same supported executable refutation comment only for the solver path, mirrors that stack-pop derivation order for the minimized core, and resets runtime-loaded PicoSAT backends before and after each exported SATCheck so `picosat_added_original_clauses()` sees only the current clause set.
- `SatClauseSetCheckAndGetCore` always calls PicoSAT with decision limit `10000`, refreshes pure-literal marks/non-pure export first, and pushes the extracted core into the caller stack without updating `core_size`. Rust exposes the same fixed-limit helper shape over its internal solver, returning a deletion-minimized core in exported-clause order while leaving `core_size` unchanged, and also has a PicoSAT-backed variant that maps solver-reported core positions through the exported-subset stack from a freshly reset solver state.
- Rust now has a runtime-loaded PicoSAT FFI boundary that opens a shared library, enables trace generation after `picosat_init`, adds sentinel-terminated clause buffers, checks `picosat_added_original_clauses`, runs `picosat_sat`, reads `picosat_coreclause` indices behind safe methods, and can reset/reinitialize the solver for the C proof-control lifecycle, with fake-ABI unit coverage for that owned solver lifecycle even when no real PicoSAT library is available. The SAT clause-set PicoSAT helpers now own the fresh-solver reset boundary around export/solve/core extraction, including cleanup after non-UNSAT results and core-extraction errors. Proof-control SATCheck and predicate-elimination gate validation can dispatch through caller-installed runtime PicoSAT backends, and the executable installs that backend when `E_RUST_PORT_PICOSAT_LIBRARY` names a PicoSAT DLL/shared library or when a bundled PicoSAT library is found next to the executable, under executable-local `lib/`, or under executable-relative `../lib/`. If no runtime library is configured or bundled, the executable falls back to the internal solver.

### Change Later

- PicoSAT integration remains the compatibility target for exact propagation/decision-limit semantics, trace generation, exported-clause accounting, and solver-reported unsat-core extraction. The internal Rust DPLL bridge is useful for search integration and tests, and the safe all/non-pure export helpers now feed a concrete runtime-loaded FFI boundary with SAT-clause-set core conversion, but making PicoSAT mandatory or bundled by default still needs packaging and reference tests with a real library; preserve the C stack-pop parent ordering when that core bridge is used.
- Solver freshness is split across C layers: `ccl_satinterface.c` exports into the solver it receives, while `che_proofcontrol.c` and `ccl_pred_elim.c` own `picosat_init`/`picosat_reset` lifecycle calls. Rust centralizes that freshness at the safe external-solver helper boundary for reusable callers; a future cleaned API should keep solver ownership explicit instead of depending on distant call-site resets.
- Rust's deletion-minimized internal core reruns the internal DPLL solver while trying to remove exported clauses. This is closer to the C proof-core surface than using every exported clause, but it is not PicoSAT trace extraction and can cost more on large UNSAT SATCheck instances; use solver-reported core extraction for default executable compatibility once runtime PicoSAT selection is covered by reference runs with a real library.
- The conjecture-frequency assignment tie-breaks are likely accidental C behavior and make the comparator mutate its input array. Keep the compatibility shim isolated so a later cleaned grounding API can switch to ordinary equality only behind reference-tested strategy compatibility.
- The SAT clause-set limit surface is signed and has different comparisons for insertion (`>=`) versus the `SatClauseSetLimitReached` macro (`==`). A future Rust-native builder should prefer a typed optional nonnegative limit once C macro compatibility is no longer needed.
- C `SatClauseAlloc` reserves a trailing zero sentinel for PicoSAT and leaves other literal slots uninitialized. Rust stores initialized vectors; if a future PicoSAT FFI path wants zero-copy DIMACS/PicoSAT buffers, keep the sentinel layout as a solver-boundary detail rather than leaking uninitialized storage into general clause code.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
