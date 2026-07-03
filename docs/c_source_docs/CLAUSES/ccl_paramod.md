<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_paramod

## Source Files

- [CLAUSES/ccl_paramod.h](../../../eprover/CLAUSES/ccl_paramod.h)
- [CLAUSES/ccl_paramod.c](../../../eprover/CLAUSES/ccl_paramod.c)

## Purpose

Interface for paramodulating termpairs into termpairs and clauses into clauses. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ParamodInfoCell`
- `ParamodInfo_p`
- `ParamodulationType`

### Macros And Constants

- `CCL_PARAMOD`
- `CheckHOUnificationConstraints(a,b,c,d)`
- `IS_NO_PARAMOD_POS`
- `PARAMOD_FROM_LENGTH_LIMIT`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ClauseOrderedParamod(TB_p bank, OCB_p ocb, ClausePos_p from,ClausePos_p into, VarBank_p freshvars)`
- `Clause_p ClauseOrderedSimParamod(TB_p bank, OCB_p ocb, ClausePos_p from,ClausePos_p into, VarBank_p freshvars)`
- `Clause_p ClauseOrderedSuperSimParamod(TB_p bank, OCB_p ocb, ClausePos_p from,ClausePos_p into, VarBank_p freshvars)`
- `Clause_p ClauseParamodConstruct(ParamodInfo_p ol_desc, ParamodulationType pm_type)`
- `Clause_p ClausePlainParamodConstruct(ParamodInfo_p ol_desc)`
- `Clause_p ClauseSimParamodConstruct(ParamodInfo_p ol_desc)`
- `Clause_p ClauseSuperSimParamodConstruct(ParamodInfo_p ol_desc)`
- `Eqn_p EqnOrderedParamod(TB_p bank, OCB_p ocb, ClausePos_p from, ClausePos_p into, Subst_p subst, VarBank_p freshvars)`
- `ParamodulationType ParamodType(char *pm_str)`
- `Term_p ClausePosFirstParamodFromSide(Clause_p from, ClausePos_p from_pos)`
- `Term_p ClausePosFirstParamodInto(Clause_p clause, ClausePos_p pos, ClausePos_p from_pos, bool no_top, ParamodulationType pm_type)`
- `Term_p ClausePosFirstParamodPair(Clause_p from, ClausePos_p from_pos, Clause_p into, ClausePos_p into_pos, bool no_top, ParamodulationType pm_type)`
- `Term_p ClausePosNextParamodFromSide(ClausePos_p from_pos)`
- `Term_p ClausePosNextParamodInto(ClausePos_p pos, ClausePos_p from_pos, bool no_top)`
- `Term_p ClausePosNextParamodPair(ClausePos_p from_pos, ClausePos_p into_pos, bool no_top, ParamodulationType pm_type)`
- `Term_p ComputeOverlap(TB_p bank, OCB_p ocb, ClausePos_p from, Term_p into, TermPos_p pos, Subst_p subst, VarBank_p freshvars)`
- `bool CheckHOUnificationConstraints(UnificationResult res, UnifTermSide exp_side, Term_p from, Term_p to)`
- `char* ParamodStr(ParamodulationType pm_type)`
- `void ParamodInfoPrint(FILE* out, ParamodInfo_p info)`

## Implementation Notes

### Internal Functions

- `check_paramod_ordering_constraint`
- `clause_pos_find_first_neg_max_lside`

### Source-Level Behavior

- `check_paramod_ordering_constraint`: Given two clause positions and an OCB, return true if the clause resulting from the described paramod-inference shall be kept for further processing. Formally, if sigma(from->clause) > sigma(into->clause), the paramodulant can be discarded. However, this check ist pretty expensive, and does not always improve performance. This function discards some of the...
- `clause_pos_find_first_neg_max_lside`: Find the first maximal negative side in the list at pos->literal.
- `ParamodStr`: Return a string representing the paramodulation type.
- `ParamodType`: Given a string encoding, return paramodulation type (or -1 if none).
- `ParamodInfoPrint`: Print a paramodulation descriptor (for debugging).
- `ClausePlainParamodConstruct`: Construct a clause via plain paramodulation according to the data in ol_desc. Return the clause, unless it's trivial tautological (then return NULL).
- `ClauseSimParamodConstruct`: Construct a clause via simultaneous paramodulation according to the data in ol_desc. Return the clause, unless it's trivial tautological (then return NULL).
- `ClauseSuperSimParamodConstruct`: Construct a clause via simultaneous paramodulation according to the data in ol_desc. Return the clause, unless it's trivial tautological (then return NULL).
- `ClauseParamodConstruct`: Construct the clause from the overlap described (and checked!) in ol_desc, either by paramodulation or simulataneous paramodulation. Return the clause. This has the implicit precondition that all variables involved are already instantiated with the mgu of ol_desc->from|from_cpos and ol_desc->into|into_pos.
- `ComputeOverlap`: Given an equation and a term position, overlap the designated side of the equation into the subterm, i.e. given s[t], u=v, return sigma(s[v]) if sigma = mgu(t,u) and sigma(u) !< sigma(v). If the operation is successful, subst will contain the mgu, and the pointer to the new term, inserted into bank, will be returned. Otherwise, subst will be unchanged and N...
- `EqnOrderedParamod`: Overlap the equation described by into from the one described by into and compute the critical pair, if one exists. Return a pointer to a critical pair, if it exists, NULL othewise. If a cp exists, subst will contain the substitution.
- `ClauseOrderedParamod`: Given two clauses, try to perform an ordered paramodulation step. Return the clause if it works, NULL otherwise.
- `ClauseOrderedSimParamod`: Perform a simultaneous ordered simultaneous paramod step (if necessary).
- `ClauseOrderedSuperSimParamod`: Perform a simultaneous ordered simultaneous paramod step (if necessary).
- `ClausePosFirstParamodInto`: Find the first potential paramod-position in clause. If no_top is true, do not select top positions of terms. Returns the term at the selected position, or NULL if no position exists. If successful and simu_paramod is true, also resets TPPotentialParamod in this and potentially following positions.
- `ClausePosNextParamodInto`: Given a position, find the next potential paramod-position. Avoid top-positions if no_top is true. Returns the term at the selected position, or NULL if no position exists.
- `ClausePosFirstParamodFromSide`: Given a clause and a position, set the position to the first side that can be used for paramodulation. Does not check strategy for efficiency reasons ClausePos*ParamodPair() should ensure that this is only called in cases were it makes sense.
- `ClausePosNextParamodFromSide`: Given a position, set the position to the next side that can be used for paramodulation. Does not check strategy for efficiency reasons. ClausePos*ParamodPair() should ensure that this is only called in cases were it makes sense.
- `ClausePosFirstParamodPair`: Given two clauses, create the first possible paramod-position from a literal in from into a literal in into. Return term paramodualated into, or NULL if no position exists.
- `ClausePosNextParamodPair`: Given two clause positions, compute the next possible paramod-position from a literal in from into a literal in into. Return term paramodualated into, or NULL if no position exists.
- `CheckHOUnificationConstraints`: Checks whether arguments are trailing on the right side of the equation (into term) and whether we are not paramodulating into the variable head of applied variable term.

### Dependencies

- `"ccl_clausecpos.h"`
- `"ccl_paramod.h"`
- `<ccl_clausesets.h>`
- `<cte_replace.h>`

### Compile-Time Conditions

- `CCL_PARAMOD`
- `ENABLE_LFHO`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for Rust indexed simultaneous/super-simultaneous and non-equational predicate paramodulation slices on 2026-06-26, and indexed plus unindexed first-order-shaped higher-order construction on 2026-07-03.

Source files reviewed: `CLAUSES/ccl_paramod.h`, `CLAUSES/ccl_paramod.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1446 lines, 22 scanned public declarations, 2 scanned internal function definitions, and 21 structured function-comment blocks.
- Interface for paramodulating termpairs into termpairs and clauses into clauses. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ComputeOverlap` intentionally leaves the successful MGU active in the caller's substitution and backtracks only rejected overlaps. Rust code should make this lifetime explicit because later literal-list copying depends on dereferencing the same substitution.
- `EqnOrderedParamod` drops positive trivial paramodulants but keeps negative trivial paramodulants so `EqnListRemoveResolved` can produce an empty clause. This is compatibility behavior, not an obvious simplification bug.
- `ClauseOrderedParamod` depends on strict maximality rechecks after substitution, not only the precomputed `EPIsMaximal` flags used to enumerate candidates.
- Generated literal flags have proof-search meaning: `EPIsPMIntoLit` survives on the new critical-pair literal, `EPFromClauseLit` marks literals copied from the source clause, and copied target-side literals have stale PM flags cleared.
- The C candidate iterators are mutable cursor APIs over clause positions and term positions; replacing them with Rust iterators is reasonable later, but exact candidate order, `no_top`, and strategy-gate behavior should be tested first.
- Rust now ports the plain, simultaneous, and super-simultaneous source/target/pair candidate order as vector-producing helpers over C-shaped `ClausePos` values, the unindexed and indexed wrappers add generated-clause insertion plus `DCParamod`/`DCSimParamod` metadata, ordinary simultaneous rewrites marked target occurrences, super-simultaneous copies the instantiated target before replacing matching occurrences, bank-backed ordering side/maximality rechecks cover KBO6 Lambda-order beta/eta preparation where these constructors own a mutable term bank, and all three generated-list constructors run `EqnListLambdaNormalize` before resolved/duplicate cleanup. In higher-order problem mode, Rust permits KBO6 first-order-shaped selected overlaps, including arrow-typed selected-variable bindings, even when unrelated copied literals contain applied-variable surfaces; indexed plain/simultaneous/super-simultaneous modes handle selected higher-order overlaps through `CsuIterator`. Selected lambda/DB/phony surfaces for unindexed paramodulation paths remain diagnostic.
- The indexed C path performs the fingerprint lookup first, then runs real unification/order gates for each candidate occurrence before iterating the stored positions and constructing clauses through `ClauseParamodConstruct`. Rust mirrors the first-order term-level gate so fingerprint false positives do not reach compact-position construction, and mirrors the higher-order indexed path by enumerating CSUs after lookup and rechecking source/target admissibility under each active binding.
- `ParamodOverlapNonEqLiterals` makes positive predicate literals participate in the same source-side cursor as equations. For a non-oriented predicate literal such as `p(a) = $true`, C enumerates both the predicate side and the `$true` side; the `$true` candidate is normally discarded later by failed unification. Rust should preserve this while matching C candidate order, even though a later explicit predicate-resolution path could skip the dead side.
- For non-equational predicate sources, `ClausePosFirst/NextParamodInto` narrows target enumeration to maximal negative left sides. The C helper signals end by returning `NULL`; Rust cursor wrappers must preserve that terminal state and must not restart from the first literal after advancing past the end.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `ParamodOverlapNonEqLiterals` and `ParamodOverlapIntoNegativeLiterals` are process-wide C strategy switches. The Rust port should keep behavior compatible initially, then move them into explicit strategy/config state when the proof-control layer is consolidated.
- Simultaneous/super-simultaneous paramodulation uses mutable term flags such as `TPPotentialParamod`. This is efficient but fragile; after parity, consider isolating the marking state from shared terms to reduce accidental cross-inference coupling.
- C's unindexed ordered-paramodulation constructor asserts against some candidates that the indexed path never passes through because it constructs via `ClauseParamodConstruct` after separate term-level checks. Rust keeps the observable indexed/unindexed split; a cleaner API should make the "checked overlap descriptor" state explicit instead of sharing assertion-heavy constructors.
- Fresh-variable normalization and per-inference `VarBankResetVCounts` are allocation-sensitive. C proof states pair `state->terms->vars` with `state->freshvars`, so temporary fresh variables carry the live variable f-codes with their original types before per-sort counters are reset. Rust now mirrors the collision-sensitive part by copying live variable codes/types into short-lived fresh banks for paramodulation, equality resolution, factoring, ArgCong, and ExtSup helpers before skipping existing stacks for generated variables; a full proof-session owner should replace these per-call banks with an explicit state-owned shadow bank and then benchmark whether reuse materially improves performance.
- Plain, simultaneous, and super-simultaneous constructors normalize generated literal lists after appending source/target copies but before removing resolved and duplicate literals. This can affect cleanup and truth/polarity side effects through `EqnMap`; keep the ordering explicit unless reference traces prove a cleaned pipeline is equivalent.
- C's higher-order paramodulation cursor is position-local: unrelated higher-order literals in the same clause are copied through if the selected overlap itself is first-order-shaped, and arrow-typed selected variables can bind through the ordinary unindexed MGU path. Rust now mirrors this with selected-term capability checks; once full higher-order unification lands, collapse the diagnostic boundary into the unifier rather than maintaining separate paramodulation preflights.
- `CheckHOUnificationConstraints` is documented as checking trailing arguments and avoiding paramodulation into applied-variable heads, but the current `ENABLE_LFHO` implementation returns `true` unconditionally. Rust mirrors the observable first-order-shaped subset instead of inventing stricter constraints; if C grows real checks, port them as explicit unifier/overlap constraints and update these diagnostics.
- Indexed paramodulation carries `pminfo->subst_is_ho` into `DPSetIsHO(DCParamod/DCSimParamod)`, but the unindexed `ComputeClauseClauseParamodulants` wrapper does not propagate a higher-order binding flag on its ordinary derivation entries. Rust preserves that distinction for compatibility; after proof-output parity is broader, decide whether unindexed higher-order bindings should be tagged uniformly or left as C behavior.
- `ClausePlainParamodConstruct`, `ClauseSimParamodConstruct`, and `ClauseSuperSimParamodConstruct` assume a successful caller-owned unifier is already active, then allocate a fresh temporary `Subst` only for normalized copying and fresh-variable tracking. Rust mirrors this split for indexed CSU construction with an explicit active `Substitution`; a future constructor could take a typed checked-overlap descriptor instead of relying on this ambient lifetime.
- Indexed `ClausePlainParamodConstruct` does not set `EPFromClauseLit` on copied source literals or `EPIsPMIntoLit` on the generated literal, unlike the unindexed ordered constructor. Rust preserves that indexed/plain distinction for compatibility; after selection and proof-output parity tests are broader, decide whether this is intentional strategy behavior or an accidental C omission.

<!-- END MANUAL REVIEW: c_source_docs -->
