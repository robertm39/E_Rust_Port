<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clausesets

## Source Files

- [CLAUSES/ccl_clausesets.h](../../../eprover/CLAUSES/ccl_clausesets.h)
- [CLAUSES/ccl_clausesets.c](../../../eprover/CLAUSES/ccl_clausesets.c)

## Purpose

Definitions dealing with collections of clauses the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClauseFunType`
- `ClauseSetCell`
- `ClauseSet_p`

### Macros And Constants

- `CCL_CLAUSESETS`
- `CLAUSECELL_DYN_MEM`
- `ClauseSetCardinality(set)`
- `ClauseSetCellAlloc()`
- `ClauseSetCellFree(junk)`
- `ClauseSetDocQuote(out, level, set, comment)`
- `ClauseSetEmpty(set)`
- `ClauseSetMoveClause(set, clause)`
- `ClauseSetStorage(set)`

### Globals

- None found in the source scan.

### Exported Functions

- `(((CLAUSECELL_DYN_MEM+EVAL_MEM((set)->eval_no))*(set)->members+\ EQN_CELL_MEM*(set)->literals)+\ PDTreeStorage(set->demod_index)+\ FVIndexStorage(set->fvindex)) ClauseSet_p ClauseSetAlloc(void)`
- `ClausePos_p ClauseSetFindEqDefinition(ClauseSet_p set, int min_arity, Clause_p start)`
- `ClauseSetExtractEntry(clause);ClauseSetInsert((set), (clause)) Clause_p ClauseSetExtractFirst(ClauseSet_p set)`
- `ClauseSetPropDocQuote((out), (level),CPIgnoreProps, (set), (comment)) bool ClauseSetVerifyDemod(ClauseSet_p demods, ClausePos_p pos)`
- `Clause_p ClauseSetExtractEntry(Clause_p clause)`
- `Clause_p ClauseSetFind(ClauseSet_p set, Clause_p clause)`
- `Clause_p ClauseSetFindBest(ClauseSet_p set, int idx)`
- `Clause_p ClauseSetFindById(ClauseSet_p set, long ident)`
- `Clause_p ClauseSetFindMaxStandardWeight(ClauseSet_p set)`
- `FunCode ClauseSetFindFreqSymbol(ClauseSet_p set, Sig_p sig, int arity, bool least)`
- `PermVector_p PermVectorCompute(ClauseSet_p set, FVCollect_p cspec, bool eliminate_uninformative)`
- `SysDate ClauseSetListGetMaxDate(ClauseSet_p *demodulators, int limit)`
- `bool ClauseSetIsUntyped(ClauseSet_p set)`
- `bool PDTreeVerifyIndex(PDTree_p root, ClauseSet_p demods)`
- `int ClauseConjectureOrder(ClauseSet_p set)`
- `long ClauseSetApplyFun(ClauseSet_p set, ClauseFunType fun)`
- `long ClauseSetCountConjectures(ClauseSet_p set, long* hypos)`
- `long ClauseSetDeleteCopies(ClauseSet_p set)`
- `long ClauseSetDeleteMarkedEntries(ClauseSet_p set)`
- `long ClauseSetDeleteNonUnits(ClauseSet_p set)`
- `long ClauseSetFVIndexify(ClauseSet_p set)`
- `long ClauseSetFilterTautologies(ClauseSet_p set, TB_p work_bank)`
- `long ClauseSetFilterTrivial(ClauseSet_p set)`
- `long ClauseSetFindCharFreqVectors(ClauseSet_p set, FreqVector_p fsum, FreqVector_p fmax, FreqVector_p fmin, FVCollect_p cspec)`
- `long ClauseSetGetSharedTermNodes(ClauseSet_p set)`
- `long ClauseSetGetTermNodes(ClauseSet_p set)`
- `long ClauseSetInsertSet(ClauseSet_p set, ClauseSet_p from)`
- `long ClauseSetMarkCopies(ClauseSet_p set)`
- `long ClauseSetMarkSOS(ClauseSet_p set, bool tptp_types)`
- `long ClauseSetMaxVarNumber(ClauseSet_p set)`
- `long ClauseSetNewTerms(ClauseSet_p set, TB_p terms)`
- `long ClauseSetParseList(Scanner_p in, ClauseSet_p set, TB_p bank)`
- `long ClauseSetPushClauses(PStack_p stack, ClauseSet_p set)`
- `long ClauseSetSplitConjectures(ClauseSet_p set, PList_p conjectures, PList_p rest)`
- `long ClauseSetTBTermPropDelCount(ClauseSet_p set, TermProperties prop)`
- `long long ClauseSetStandardWeight(ClauseSet_p set)`
- `void ClauseSetAddAxiomSymbolDistribution(ClauseSet_p set, long *dist_array)`
- `void ClauseSetAddConjSymbolDistribution(ClauseSet_p set, long *dist_array)`
- `void ClauseSetAddSymbolDistribution(ClauseSet_p set, long *dist_array)`
- `void ClauseSetAddTypeDistribution(ClauseSet_p set, long *type_array)`
- `void ClauseSetComputeFunctionRanks(ClauseSet_p set, long *rank_array, long* count)`
- `void ClauseSetDefaultWeighClauses(ClauseSet_p set)`
- `void ClauseSetDelProp(ClauseSet_p set, FormulaProperties prop)`
- `void ClauseSetDeleteEntry(Clause_p clause)`
- `void ClauseSetDerivationStackStatistics(ClauseSet_p set)`
- `void ClauseSetDocInital(FILE* out, long level, ClauseSet_p set)`
- `void ClauseSetDocQuote(FILE* out, long level, ClauseSet_p set, char* comment)`
- `void ClauseSetFree(ClauseSet_p junk)`
- `void ClauseSetFreeClauses(ClauseSet_p set)`
- `void ClauseSetGCMarkTerms(ClauseSet_p set)`
- `void ClauseSetIndexedInsert(ClauseSet_p set, FVPackedClause_p newclause)`
- `void ClauseSetIndexedInsertClause(ClauseSet_p set, Clause_p newclause)`
- `void ClauseSetIndexedInsertClauseSet(ClauseSet_p set, ClauseSet_p source)`
- `void ClauseSetInsert(ClauseSet_p set, Clause_p newclause)`
- `void ClauseSetMarkMaximalTerms(OCB_p ocb, ClauseSet_p set)`
- `void ClauseSetPDTIndexedInsert(ClauseSet_p set, Clause_p newclause)`
- `void ClauseSetPrint(FILE* out, ClauseSet_p set, bool fullterms)`
- `void ClauseSetPrintPrefix(FILE* out, char* prefix, ClauseSet_p set)`
- `void ClauseSetPropDocQuote(FILE* out, long level, FormulaProperties prop, ClauseSet_p set, char* comment)`
- `void ClauseSetRemoveEvaluations(ClauseSet_p set)`
- `void ClauseSetSetProp(ClauseSet_p set, FormulaProperties prop)`
- `void ClauseSetSetTPTPType(ClauseSet_p set, FormulaProperties type)`
- `void ClauseSetSort(ClauseSet_p set, ComparisonFunctionType cmp_fun)`
- `void ClauseSetSortLiterals(ClauseSet_p set, ComparisonFunctionType cmp_fun)`
- `void ClauseSetTSTPPrint(FILE* out, ClauseSet_p set, bool fullterms)`
- `void ClauseSetTermSetProp(ClauseSet_p set, TermProperties prop)`
- `void EqAxiomsPrint(FILE* out, Sig_p sig, bool single_subst)`

## Implementation Notes

### Internal Functions

- `clause_set_extract_entry`
- `eq_func_axiom_print`
- `eq_pred_axiom_print`
- `print_var_pattern`
- `tptp_eq_func_axiom_print`
- `tptp_eq_pred_axiom_print`

### Source-Level Behavior

- `print_var_pattern`: Print a template for a function/predicate symbol.
- `eq_func_axiom_print`: Print the LOP substitutivity axiom(s) for a function symbol.
- `eq_pred_axiom_print`: Print the LOP substitutivity axiom(s) for a predicate symbol.
- `tptp_eq_func_axiom_print`: Print the TPTP substitutivity axiom(s) for a function symbol.
- `tptp_eq_pred_axiom_print`: Print the TPTP substitutivity axiom(s) for a predicate symbol.
- `clause_set_extract_entry`: Remove a plain clause from a plain clause set.
- `ClauseSetAlloc`: Allocate an empty clause set that uses SysDate for (logical) time-keeping.
- `ClauseSetFreeClauses`: Delete all clauses in set.
- `ClauseSetFree`: Delete a clauseset.
- `ClauseSetStackCardinality`: Assume stack is a stack of clause sets. Return the number of clauses in all the sets.
- `ClauseSetGCMarkTerms`: Mark all terms in the clause set for the garbage collection.
- `ClauseSetInsert`: Insert a clause as the last clause into the clauseset.
- `ClauseSetInsertSet`: Move all clauses from from into set (leaving from empty, but not deleted).
- `ClauseSetPDTIndexedInsert`: Insert a demodulator into the set and the sets index.
- `ClauseSetIndexedInsert`: Insert an FVPackedClause clause into the set, taking care od of all existing indexes.
- `ClauseSetIndexedInsertClause`: Insert a plain clause into the set, taking care od of all existing indexes.
- `ClauseSetIndexedInsertClauseSet`: Update the standard weight of all clauses in source and insert them into set (and the indices of set).
- `ClauseSetExtractEntry`: Remove a (possibly indexed) clause from a clause set.
- `ClauseSetExtractFirst`: Extract the first element of the set and return it. Return NULL if set is empty.
- `ClauseSetDeleteEntry`: Delete a clause from the clause set.
- `ClauseSetFindBest`: Find the best clause (i.e. the clause with the smallest evaluation).
- `ClauseSetPrint`: Print the clause set to the given stream.
- `ClauseSetTSTPPrint`: Print the clause set in TSTP format to the given stream.
- `ClauseSetPrintPrefix`: Print the clause set, one clause per line, with prefix prefix on each line.
- `ClauseSetSort`: Sort a clause set according to the comparison function given. Note: This is unnecssarily inefficient for evaluated clauses! Reimplement if you need to use it for large evaluatied sets!
- `ClauseSetSetProp`: Set prop in all clauses in set.
- `ClauseSetDelProp`: Delete prop in all clauses in set.
- `ClauseSetSetTPTPType`: Set TPTP type in all clauses in set.
- `ClauseSetMarkCopies`: Mark clauses that are equivalent (modulo ClauseCompareFun) to clauses that occur earlier in set. Returns number of marked clauses.
- `ClauseSetDeleteMarkedEntries`: Remove all clauses with property CPDeleteClause set. Returns number of deleted clauses.
- `ClauseSetDeleteCopies`: Delete all but one occurence of a clause in set.
- `ClauseSetDeleteNonUnits`: Remove all non-empty-non-unit-clauses from set, return number of clauses eliminated.
- `ClauseSetGetTermNodes`: Count the nodes of terms in the clauses of set as though they were unshared.
- `ClauseSetMarkSOS`: Mark Set-of-Support clauses in set with CPIsSOS. Return size of SOS.
- `ClauseSetTermSetProp`: Set prop in all term nodes in clause set.
- `ClauseSetTBTermPropDelCount`: Delete prop in all term cells, return number of props encountered
- `ClauseSetGetSharedTermNodes`: Return the number of shared term nodes used by set.
- `ClauseSetParseList`: Parse a list of clauses into the set. Clauses are not evaluated. Returns number of clauses parsed.
- `ClauseSetMarkMaximalTerms`: Orient all literals and mark all maximal terms and literals in the set.
- `ClauseSetSortLiterals`: Sort literals in all clauses by cmp_fun.
- `ClauseSetListGetMaxDate`: Return the oldest date of the first limit elements from set of demodulators in the array demodulators.
- `ClauseSetFind`: Given a clause and a clause set, try to find the clause in the set. This is only useful for debugging, as usually clause should know about the set it is in!
- `ClauseSetFindById`: Given a clause ident and a clause set, try to find the clause in the set.
- `ClauseSetRemoveEvaluations`: Remove all evaluations from the clauses in set.
- `ClauseSetApplyFun`: Apply fun to all clauses in set. Return the sum of the return values.
- `ClauseSetFilterTrivial`: Given a clause set, remove all trivial tautologies from it. Return number of clauses removed.
- `ClauseSetFilterTautologies`: Given a clause set, remove all tautologies from it. Return number of clauses removed.
- `ClauseSetFindMaxStandardWeight`: Return a pointer to a clause with the largest standard weight among clauses in set (or NULL if set is empty).
- `ClauseSetFindEqDefinition`: If set contains an equality definition at or after start, return the potential matching side (as a reduced clause position), otherwise NULL.
- `ClausesSetDocInital`: If level >= 2, print all clauses as axioms.
- `ClauseSetPropDocQuote`: Quote all clauses in set for which all props are set.
- `ClauseSetVerifyDemod`: Return true if pos->clause is in clause set, is a demodulator, and if pos describes a potential maximal side in clause.
- `PDTreeVerifyIndex`: Check if all clauses in index are in demod as well.
- `EqAxiomsPrint`: Print the equality axioms (symmetry, transitivity, refexivity, substitutivity) for the given signature.
- `ClauseSetAddSymbolDistribution`: Count the occurrences of function symbols in set.
- `ClauseSetAddTypeDistribution`: Count the occurrences of types of function symbols in set.
- `ClauseSetAddConjSymbolDistribution`: Count the occurrences of function symbols in conjectures in set.
- `ClauseSetAddAxiomSymbolDistribution`: Count the occurrences of function symbols in non-conjectures in set.
- `ClauseSetComputeFunctionRanks`: Assign to each function symbol a uniq number based on its position in the clause set.
- `ClauseSetFindFreqSymbol`: Find the most/least frequent non-special, non-predicate symbol of the given arity in the clause set.
- `ClauseSetMaxVarNumber`: Return the largest number of variables occurring in any clause.
- `ClauseSetFindCharFreqVectors`: Compute the characteristic frequency vectors for set. Vectors are re-initialized. Returns number of clauses in set.
- `PermVectorCompute`: Given a clause set and parameters for an index, compute a suitable permutation vector (may be NULL if the parameters do not call for a permutation vector!)
- `ClauseSetFVIndexify`: Remove all clauses from set and insert them again as indexed clauses. Return number of clauses in set.
- `ClauseSetNewTerms`: Substitute all clause in set with otherwise identical copies taking terms from the new termbank.
- `ClauseSetSplitConjectures`: Find all (real or negated) conjectures in set and sort them into conjectures. Collect the rest in rest. Return number of conjectures found.
- `ClauseSetStandardWeight`: Return the sum of the standardweight of all clauses in set.
- `ClauseSetDerivationStackStatistics`: Compute and print the stack depth distribution of the clauses in set.
- `ClauseSetPushClauses`: Push all clauses in set onto stack. Return number pushed.
- `ClauseSetDefaultWeighClauses`: Set the (standard) weight in all clauses in set.
- `ClauseSetCountConjectures`: Count and return number of conjectures (and negated_conjectures) in set. Also find number of hypotheses, and add it to *hypos.
- `ClauseConjectureOrder`: Return the maximal order of the symbols that appear in the conjecture.

### Dependencies

- `"ccl_clausesets.h"`
- `<ccl_derivation.h>`
- `<ccl_fcvindexing.h>`
- `<ccl_inferencedoc.h>`
- `<ccl_pdtrees.h>`
- `<ccl_tautologies.h>`
- `<clb_objtrees.h>`
- `<clb_plist.h>`

### Compile-Time Conditions

- `CCL_CLAUSESETS`
- `NDBUG`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_clausesets.h`, `CLAUSES/ccl_clausesets.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 2725 lines, 70 scanned public declarations, 6 scanned internal function definitions, and 72 structured function-comment blocks.
- Clause-set container logic. Processed/unprocessed transitions and list membership are observable through proof-state algorithms.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ClauseSetInsert`, extraction, `ClauseSetFindBest`, and `ClauseSetRemoveEvaluations` keep the evaluation-index roots in sync with each clause's owned `EvalCell`. C reaches and unlinks the owning clause through intrusive pointers; Rust preserves the root-clearing order and `EvalCompare` key semantics with safe sorted roots plus private clause slots that resolve evaluation objects and remove selected clauses without scanning or shifting the set. Bounded slot compaction rebuilds both internal lookup maps without changing logical set order.
- `ClauseSetPrint` and `ClauseSetPrintPrefix` append the newline at the set loop, not in `ClausePrint`; the prefix variant always calls `ClausePrint(..., true)` regardless of the non-prefix caller's `fullterms` argument. Rust preserves this shape in default LOP plus explicit LOP/TPTP/TSTP set and prefix string helpers.
- `ClauseSetPrint` is documented with only term-output globals and `ClauseSetPrintPrefix` documents a read-only format global, but both reach the process-global `OutputFormat` through `ClausePrint`; TSTP clause printing also observes the process-global problem type. Rust keeps those dependencies explicit through output-format and problem-type parameters.
- `ClauseSetParseList` loops while `ClauseStartsMaybe` is true, so a bare identifier after the intended clause list is treated as another possible clause and produces a syntax error instead of acting as a clean terminator. Rust preserves that token-start behavior over the simple clause parser; callers that need sentinels should use a token that cannot start a clause or add an explicit higher-level boundary.
- `ClauseSetMarkMaximalTerms` is a straight set-order loop over `ClauseMarkMaximalTerms`, so it inherits each clause's orientation/maximality cache contract and performs no set-level cache validation. Rust preserves that layering and now exposes a bank-backed set loop for callers that can supply the mutable owner bank needed by higher-order KBO6 `LAMBDA_ORDER`; indexed or shared clause-set owners should keep cache invalidation at the clause/literal boundary.
- `ClauseSetTSTPPrint` is a set-order loop that calls `ClauseTSTPPrint(..., complete=true)` and then writes one newline for every clause. C inherits hidden global problem-type/output-format behavior from the clause printer; Rust preserves the text shape through an explicit term bank and problem type, including the typed first-order formula-closure branch, and returns a diagnostic if a clause reaches the still-deferred higher-order formula-aware branch.
- `EqAxiomsPrint` switches on the process-global output format: TPTP emits old `input_clause` syntax, TSTP aborts as unsupported, and every other format falls back to LOP-style clauses. Rust exposes the format as an explicit argument and preserves the single-substitution expansion, while keeping TSTP as a diagnostic until a compatibility target needs that fatal-error path.
- `ClauseSetFind` is a raw pointer-identity check and `ClauseSetVerifyDemod` builds on that identity before checking demodulator shape and rejecting the right side of an oriented equality. Rust preserves the identity check for borrowed clauses and exposes the demodulator-side predicate over borrowed set entries; stable handle-backed `ClausePos` ownership should replace cloned positions when full indexed demodulator search is ported.
- `ClauseSetFVIndexify` extracts clauses from the front into a stack and then pops them back into the set as indexed clauses, so the final set order is reversed. Rust preserves that LIFO reinsertion and `CPIsSIndexed` marking for both the transition explicit-anchor helper and the owned optional FV-anchor path.
- `ClauseSetIndexedInsertClauseSet` recomputes each source clause's standard weight before indexed insertion and keeps source iteration order. Rust preserves that behavior through explicit-anchor wrappers and the owned optional FV-anchor API.
- `ClauseSetExtractEntry` deletes `CPIsSIndexed` clauses from `set->fvindex` and clears the property during extraction. Rust mirrors that lifecycle for owned FV anchors while keeping the explicit-anchor helpers as transition APIs until full proof-state set ownership is wired.
- `ClauseSetStorage` estimates clause cells, evaluation cells, literal cells, demodulator-index storage, and FV-index storage with C's constant-memory branch. Rust now preserves the clause/evaluation/literal/demodulator-index/FV-index portions for cleanup-limit accounting when the corresponding optional indexes are owned by the set.
- `ClauseSetDerivationStackStatistics` prints a derivation-stack-depth histogram directly to stdout, counts clauses without derivation stacks in bucket zero, uses a `PDArray` with initial/grow size 8, and prints every allocated bucket including zero counts. Rust exposes this as an output-aware writer that preserves the bucket allocation and six-decimal average formatting.
- `ClauseSetPropDocQuote` filters with the same all-bits property query as `ClauseQueryProp`, so `CPIgnoreProps` intentionally quotes every clause. Rust mirrors that filter for supported final proof-search documentation quotes before the result banner.

### Change Later

- C stores `set`, `pred`, and `succ` raw pointers in every `ClauseCell`, which enforces one-set membership by convention and makes arbitrary extraction constant-time, but exposes ownership and unlink ordering to every caller. Rust's private sparse slots preserve the performance property safely for current set-local operations; once `ClausePos`, derivations, and global indexes need long-lived clause identity, replace the private slot numbers with typed generational clause handles rather than exporting C-shaped back-pointers.
- `ClauseSetExtractEntry` assumes `CPIsSIndexed` implies `clause->set->fvindex` is valid and calls `FVIndexDelete` unconditionally. Rust preserves the indexed-clause lifecycle for owned anchors, but later stable clause-handle APIs should make index membership explicit enough to prevent stale indexed bits, double deletes, or missing anchors from corrupting index counts.
- Rust `ClauseSet` now owns an optional demodulator `PdTree`, marks `CPIsDIndexed` on C-style indexed insertion, inserts/deletes dated left/right-side demodulator occurrences so the trie maintains C-shaped size/date constraints and can recover compact clause-id/side candidates in recorded branch order, exposes conservative per-node constraint and search-path pruning plus compact variable type-UID, variable-edge weight adjustment, and repeated-variable query-slice checks for current rewrite/unit stand-ins when all demodulators are indexed, records demodulator search-init match attempts, query term code/spans/weight/date, active-search state, active cursor state, traversal-order state, and compact successful-child visit accounting for indexed C search stand-ins, maintains first-in-set clause-id positions so rewrite/unit candidate lookup does not rebuild a whole-set map per query, and includes `PDTreeStorage(set->demod_index)` in `ClauseSetStorage`. Full `PDTreeFindNextDemodulator` search should replace the compact id bridge with live stable `ClausePos` leaf handles and incremental traversal/backtracking state.
- C clause sets reach each clause's owner bank implicitly through the literals. Rust currently has immutable-bank and mutable-bank maximal-marking entry points; collapse that split only after clause ownership can provide the same owner-bank context without passing it through every caller.
- `ClauseSetDerivationStackStatistics` couples diagnostic rendering to stdout, prints fixed allocation buckets rather than just observed depths, and divides by the clause count without guarding empty sets. Keep the compatibility writer available, but a cleaned statistics API should return a structured histogram and make empty-set average spelling explicit.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
