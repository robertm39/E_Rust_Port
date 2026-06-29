<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_ho_inferences

## Source Files

- [CONTROL/cco_ho_inferences.h](../../../eprover/CONTROL/cco_ho_inferences.h)
- [CONTROL/cco_ho_inferences.c](../../../eprover/CONTROL/cco_ho_inferences.c)

## Purpose

Declarations of functions that implement higher-order inferences that are non-essential to superposition. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `DEFAULT_RENAMING_LIMIT`
- `FAIL_ON(cond)`
- `PROOF_DEPTH(c)`
- `PROOF_SIZE(c)`
- `PtrPairAlloc()`
- `PtrPairFree(junk)`
- `TermArgAlloc(n)`
- `TermArgFree(arg, n)`

### Globals

- None found in the source scan.

### Exported Functions

- `bool BooleanSimplification(Clause_p cl)`
- `bool ImmediateClausification(Clause_p cl, ClauseSet_p store, ClauseSet_p archive, VarBank_p fresh_vars, bool fool_unroll)`
- `bool NormalizeEquations(Clause_p cl)`
- `bool ResolveFlexClause(Clause_p cl)`
- `void ClausePruneArgs(Clause_p cl)`
- `void ClauseSetRecognizeChoice(IntMap_p choice_syms, ClauseSet_p set, ClauseSet_p archive)`
- `void ComputeHOInferences(ProofState_p state, ProofControl_p control, Clause_p renamed_clause, Clause_p orig_clause)`
- `void PreinstantiateInduction(FormulaSet_p forms, ClauseSet_p cls, ClauseSet_p archive, TB_p bank)`

## Implementation Notes

### Internal Functions

- `mk_ptr_pair`

### Source-Level Behavior

- `mk_ptr_pair`: Create a pointer pair on the heap
- `instantiate_w_abstractions`: Find abstraction for the variable var in orig_cl and store the resulting clause in res
- `do_abstract`: Replace arg with DB variable 0 (appropriately shifted) in t
- `abstract_arg`: Construct an abrastraction %x. lhs[x] = rhs[x] where lhs[x] is lhs in which arg is replaced by x (similarly rhs[x]).
- `store_abstraction`: Adds the calculated abstraction to the store
- `store_abstraction_form`: If the formula is of the shape Q1[X]. Q2[Y].... Qn[Z]: f where Qi is a quantifier, store abstraction for every quantifier.
- `store_abstraction_cl`: Stores the computed inference with the given derivation code in the temporary store for the newly infered clauses.
- `set_proof_object`: Stores the computed inference with the given derivation code in the temporary store for the newly infered clauses.
- `store_result`: Stores the computed inference with the given derivation code in the temporary store for the newly infered clauses.
- `fresh_pattern_w_ty`: Given an applied variable s, create a fresh variable applied to bound variables representing arguments of s, whose return type is ty
- `fresh_pattern`: Like fresh_pattern_w_ty but copies the return type from t
- `close_for_appvar`: Analyze the arguments of applied variable and generate a lambda prefix for the matrix that corresponds to each argument of the lambda var
- `apply_pattern_vars`: Apply fresh variables (applied to bound ones that correspond to arguments of appvar) to head.
- `mk_prim_enum_inst`: Create an instance of clause and set the proof object for primitive enumeration.
- `remove_constant_args`: For each variable in var_occs, mark the indexes of arguments that always occur with the same value.
- `remove_repeated_args`: For each variable in var_occs remove the argument with index i if there is another argument with index j such that for each occurence arugments at i and j are the same.
- `compute_removal_subst`: Based on data in var_removed_args (containing indexes of arguments to be removed), create a substitution removing all the arguments. Returns true if at least one argument is removed.
- `find_disagreements`: Stores the computed inference with the given derivation code in the temporary store for the newly infered clauses.
- `advance_eq_fact_pos`: Given an *initialized* clause position pos, find the next one which can take part in ExtEqFact inference
- `do_ext_eq_fact`: Given an *initialized* clause position pos, find the next one which can take part in ExtEqFact inference
- `do_ext_sup`: Performs ExtEqRes inference.
- `do_ext_sup_from`: Performs ExtSup inferences with the given clause used as 'from' partner
- `do_ext_sup_into`: Performs ExtSup inferences with the given clause used as 'intos' partner
- `find_choice_triggers`: Find subterms of t that are of the form ch(s) where ch is in choice_syms. Store the subterm on stack triggers.
- `do_mk_choice_inst`: Given a term whose head is a defined choice symbol, instantiate the corresponding choice axiom with the argument of the choice term.
- `inst_choice`: Instantiate choice axiom for choice code with trigger and its negation.
- `mk_new_choice`: Given a type ty create a new clause representing choice axiom. Store the new clause in choice syms map and in the archive.
- `term_drop_last_arg`: Removes the last argument of a term. Assumes there is at least one argument.
- `mk_leibniz_instance`: Bind variable to binding and add the corresponding instance to the proof state.
- `ComputeNegExt`: Computes all possible NegExt inferences with the given clause. NegExt is described by s != t \/ C s (sk (free_vars(s,t))) != t (sk (free_vars(s,t))) \/ C where s != t is a maximal literal.
- `ComputeArgCong`: Computes all possible ArgCong inferences with the given clause. ArgCong is described by s = t \/ C s FRESH_VAR = t FRESH_VAR \/ C
- `ComputePosExt`: Computes all possible PosExt inferences with the given clause. PosExt is described by s X = t X \/ C s = t \/ C where s = t is a maximal literal and X does not appear in s and t and C.
- `InferInjectiveDefinition`: If clause postulates injectivity of some symbol add the definition of inverse to the proof state.
- `ComputeExtSup`: Computes abstracting variant of superposition rule.
- `ComputeExtEqRes`: Computes abstracting variant of equality resolution.
- `ComputeExtEqFact`: Computes abstracting variant of equality factoring.
- `NormalizeEquations`: Lifts nested equalities to the literal equality level, and removes nested $nots.
- `ImmediateClausification`: Performs dynamic clausfication of equivalences.
- `EliminateLeibnizEquality`: Find a subclause of C of the form X sn | ~X tn and generate two series of instances C{X |-> %xn. x_i != s_i} and C{X |-> %xn. x_i = t_i}.
- `PrimitiveEnumeration`: Instantiate clauses with primitive substitutions -- imitations of logical symbols.
- `BooleanSimplification`: Performs boolean simplification and returns true if formula becomes redundant.
- `ResolveFlexClause`: If a clause contains only negative disequations of the form X @ s_n != Y @ t_n, derive the empty clause
- `InstantiateChoiceClauses`: Scan the clause for term of the form (f t) where f is a defined choice symbol and instantiate the saved choice axioms with t and negation thereof
- `PreinstantiateInduction`: Compute all induction triggers from the original clause set and instantiate clauses that have variables of the correct type.
- `ComputeHOInferences`: Computes all registered HO inferences.

### Dependencies

- `"cco_ho_inferences.h"`
- `<ccl_tcnf.h>`
- `<che_proofcontrol.h>`
- `<cte_lambda.h>`

### Compile-Time Conditions

- `ENABLE_LFHO`
- `NDEBUG`
- `USE_SYSTEM_MEM`

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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_ho_inferences.h`, `CONTROL/cco_ho_inferences.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 2762 lines, 11 scanned public declarations, 1 scanned internal function definitions, and 46 structured function-comment blocks.
- Higher-order inference control; keep lambda/type-bank assumptions aligned with `TERMS` higher-order modules.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `BooleanSimplification` is partially staged in Rust through `src/clauses/clausefunc.rs` and called from `forward_contract_keep` at the C call site. The port covers decoded two-argument Boolean formula simplification, unary decoded `and`/`or` constant-to-DB-lambda cases, closed-lambda decoded quantifier matrix removal, true-literal redundancy detection, superfluous-literal cleanup, and `DCNormalize` derivation metadata.
- `NormalizeEquations` is staged in Rust as `clause_normalize_equations` and called from the higher-order `ForwardModifyClause` hook. The port covers the C `$true`-side swap, encoded `$not` stripping, encoded `$eq`/`$neq` lifting to literal sides, `$false` polarity normalization, equational-literal property recomputation, stale orientation/maximality property clearing, superfluous-literal cleanup, cached-weight refresh, and `DCNormalize` derivation metadata.
- `ResolveFlexClause` is staged in Rust as `clause_resolve_flex_clause` and called from the higher-order `forward_contract_keep` hook. The port covers the C top-level-free-variable test for negative equality literals, the predicate-literal sign bookkeeping, the predicate-variable/equality conflict rule, empty-clause replacement, and `DCFlexResolve` derivation metadata.
- `ClausePruneArgs` is staged in Rust as `clause_prune_args` and called from the higher-order `ForwardModifyClause` hook when `--prune-args` is active. The port covers applied/naked free-variable occurrence collection, C-style constant-argument and repeated-argument marking by term identity, generated DB-lambda replacement bindings, instantiated term-bank reinsertion, beta normalization of generated pruning applications, resolved/duplicate literal cleanup, cached-weight refresh, and `DCPruneArg` derivation metadata.
- `ComputeArgCong` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers positive-literal and `AllLits`/`MaxLits` gating, fresh-variable prefix accumulation, generated equality insertion before copied residual literals, beta normalization for represented generated literal lists, tmp-store insertion, and `DCArgCong` proof metadata.
- `ComputeNegExt` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers negative-literal and `AllLits`/`MaxLits` gating, per-prefix typed Skolem argument generation over variables collected from the selected literal, cumulative left/right applications, generated negative equality insertion before copied residual literals, beta normalization for represented generated literal lists, `neg_ext_count`, tmp-store insertion, and `DCNegExt` proof metadata.
- `ComputePosExt` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers the C `neg_ext` gate around the positive-extensionality call, positive equational `AllLits`/strictly-maximal `MaxLits` filtering, repeated shared trailing free-variable prefix dropping, occurrence checks over earlier literal arguments and all other literals, generated positive equality insertion before copied residual literals, non-normalizing generated-list behavior, tmp-store insertion, and `DCPosExt` proof metadata.
- `InferInjectiveDefinition` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers the `inverse_recognition` gate, C `ClauseRecognizeInjectivity` clause shape, inverse typed-Skolem generation, direct tmp-store insertion, pure-injectivity marking, source TPTP/SOS/proof-depth/proof-size propagation, and `DCInvRec` derivation metadata attached by the recognizer.
- `EliminateLeibnizEquality` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers the proof-depth gate, applied predicate-variable literal scan by polarity, opposite-polarity head detection, per-argument occurrence guard, DB-lambda equality/disequality binding construction, instantiated higher-order literal copying, represented beta normalization, resolved/duplicate literal cleanup, `NormalizeEquations` before storage, tmp-store insertion, and `DCLeibnizElim` proof metadata with depth increment 1.
- `PrimitiveEnumeration` is staged in Rust as the higher-order prefix of `proof_state_generate_new_clauses`. The port covers the proof-depth gate, per-clause applied-variable head de-duplication, fresh pattern generation over DB-lambda binders, `Neg`/`And`/`Or`/`Eq`/`Full` mode candidates, unconditional true/false candidates, `Pragmatic` repeated-argument equality/disequality and predicate projection candidates, `LogSymbol` logical connective and quantifier candidates, instantiated higher-order literal copying, represented beta normalization, resolved/duplicate literal cleanup, `NormalizeEquations`, post-proof-object Boolean simplification, tmp-store insertion, and `DCPrimEnum` proof metadata with depth increment 1.
- `ClauseSetRecognizeChoice` is staged in Rust through proof-state initialization. The port covers the C higher-order plus `inst_choice_max_depth >= 0` gate, beta-normal represented `ClauseRecognizeChoice` shape, duplicate choice-symbol rejection, literal-left normalization, and `choice_opcodes` storage for later `InstantiateChoiceClauses`.

### Change-Later Observations

- `BooleanSimplification` lives in this higher-order control unit, but `cco_forward_contraction.c` calls it unconditionally for every clause after `ForwardModifyClause`. Keep that cross-module dependency while matching C behavior; after compatibility is secured, consider moving shared decoded-Boolean simplification into a formula/clause normalization module with the higher-order callers depending on that lower-level API.
- `NormalizeEquations` is syntactic rather than a general formula normalizer: it only inspects predicate literals with an encoded Boolean/equality term on the left and `$true` on the right, plus the preliminary `$true`-on-left swap. Keep that surface for compatibility, but a later formula layer should make the accepted encoded shapes explicit.
- `NormalizeEquations` rewrites `$false` on either lifted side to `$true` and toggles polarity, then may swap a `$true` left side to the right. Rust mirrors this sign choreography; revisit only with higher-order proof-search traces because simplifying it would alter literal polarity and equational-literal flags.
- C can record two `DCNormalize` derivation entries when `ClauseRemoveSuperfluousLiterals` removes something inside `NormalizeEquations` before `NormalizeEquations` pushes its own derivation. Rust preserves that possible double-normalize shape through the shared cleanup helper.
- C does not explicitly refresh `clause->weight` after `NormalizeEquations`; Rust refreshes the cached weight after mutated literal sides so current Rust clause-set and comparison invariants stay valid. Revisit only if stale C weights prove observable in reference traces.
- `ResolveFlexClause` is broader than its comment: it accepts naked top-level free variables as well as applied free variables, and it also reasons about non-equational predicate literals by remembered sign. Keep this for compatibility, but a later higher-order inference API should name the accepted literal classes explicitly.
- If called directly on an already empty clause, `ResolveFlexClause` would vacuously succeed and push a flex-resolution derivation, although `forward_contract_keep` short-circuits empty clauses before calling it. Rust keeps the call-site short-circuit and the helper's all-literals predicate shape; avoid exposing direct empty-clause flex resolution as a public semantic rule without reference tests.
- `ResolveFlexClause` drops the literal list and recomputes polarity counts but does not explicitly refresh `clause->weight`. Rust refreshes the cached weight after replacement to preserve current Rust indexing/comparison invariants; revisit only if full C trace comparison proves stale empty-clause weights are observable.
- `ClausePruneArgs` stops scanning removable argument positions at the first missing argument in the first occurrence. That means later supplied positions in partially applied variables are ignored if an earlier position is absent. Rust mirrors this contiguous-prefix behavior; a later cleaned API should make partial-application treatment explicit if compatibility permits.
- `ClausePruneArgs` uses pointer identity, not structural equality, for constant and repeated argument detection. Rust intentionally uses term-handle identity for these tests; replacing it with structural comparison would change both performance and proof-search behavior.
- `ClausePruneArgs` only removes constant arguments when the argument term is DB-closed, but repeated-argument pruning does not require DB-closed terms. Keep this asymmetry until higher-order trace comparisons cover lambda-open terms.
- C removes/reshares pruned literals through `EqnListLambdaNormalize`, i.e. full beta plus eta normalization. Rust currently applies the beta-normalization subset needed for generated pruning bindings; full eta behavior remains tied to the broader `cte_lambda` port.
- C does not explicitly refresh `clause->weight` after `ClausePruneArgs`; Rust refreshes cached counts and weight after rewritten literal cleanup to preserve current Rust clause invariants. Revisit only if stale C weights prove observable in reference traces.
- `ComputeArgCong` accumulates fresh arguments and emits one clause for each nonempty prefix application, not only a fully saturated application. Rust preserves that sequence and reuses the earlier fresh variable handles in later prefixes.
- `ComputeHOInferences` checks `neg_ext != NoLits` before calling both negative and positive extensionality, so `pos_ext` alone is ignored by the checked C source. Rust mirrors that gate by running represented PosExt only when `neg_ext` is also enabled; after compatibility traces exist, decide whether `pos_ext` should become an independent option.
- `ComputeNegExt` creates a fresh typed Skolem symbol for each nonempty prefix application instead of reusing one witness across all prefixes. Rust preserves that emitted-clause and symbol sequence for compatibility; revisit only with higher-order traces that cover generated Skolem names and proof shape.
- `term_drop_last_arg` reconstructs a PosExt prefix with a widened arrow type built from the dropped argument type and the current residual type, and returns the phony-application head directly for binary phony applications. Rust preserves that type-shaping behavior for represented PosExt; later term-bank APIs should make prefix/witness type ownership clearer after higher-order traces pin down sharing effects.
- `ComputeArgCong` and `ComputeNegExt` call `EqnListLambdaNormalize` on generated literals. Rust currently applies the represented beta-normalization subset for generated ArgCong/NegExt clauses; full eta normalization should be revisited with the broader `cte_lambda` port.
- `ComputePosExt` does not call `EqnListLambdaNormalize` after dropping trailing arguments, unlike ArgCong/NegExt. Rust preserves that non-normalizing path; revisit only after higher-order traces show whether lambda-shaped PosExt outputs need postprocessing outside this inference.
- `ClauseRecognizeInjectivity` attaches `DCInvRec` derivation metadata itself, while `InferInjectiveDefinition` only inserts the generated clause and the C proof-documentation call remains a TODO. Rust mirrors that layering; a later proof-doc owner should separate compatibility rendering from the recognizer's clause construction.
- `ClauseSetRecognizeChoice` accepts an `archive` argument and its comment says recognized clauses are moved there, but the checked C implementation only records existing clause pointers in `choice_opcodes`. Rust stores owned clause copies in the map until stable proof-state clause handles can represent that pointer aliasing directly.
- `EliminateLeibnizEquality` only records non-equational predicate literals whose left side is an applied free variable; naked predicate variables are not considered by this inference despite the broad prose description of `X s_n`. Rust preserves that accepted literal class until reference traces show whether a cleaned rule should include naked Boolean variables.
- `EliminateLeibnizEquality` builds a full DB-lambda binding over all applied arguments for each eligible argument position, using a DB index based on the argument's suffix position. Rust preserves the generated binding shape because changing it can alter beta-normalized literal order and printed proof terms.
- `mk_leibniz_instance` runs `NormalizeEquations` before `store_result`, so a generated clause can carry a `DCNormalize` derivation entry before the final `DCLeibnizElim` entry. Rust preserves that derivation ordering for represented generated clauses.
- `store_result` records `DCArgCong`, `DCNegExt`, and `DCPosExt` with depth increment 0, so generated ArgCong/NegExt/PosExt clauses can keep the same proof depth as their parent. `DCLeibnizElim` uses depth increment 1. Rust preserves these proof-depth shapes even though a later proof metric API might prefer uniformly increasing inference depth.
- `PrimitiveEnumeration` scans both sides only when the literal left side has Boolean type, so a right-side applied variable is skipped if the left side is not Boolean. It also de-duplicates by the applied-variable head f-code across the whole clause. Rust preserves both gates; revisit only with traces showing this is accidental rather than proof-search policy.
- `PrimitiveEnumeration` emits true and false substitutions in every mode, including the narrow `Neg`, `And`, `Or`, `Eq`, and `LogSymbol` modes. Rust keeps the unconditional candidates because removing them changes generated-clause counts and possible refutations.
- C gathers distinct argument return types for `EqMode`/`FullMode` in a pointer tree, so candidate order follows type pointer identity rather than source-argument order. Rust mirrors that with `type_identity_cmp`; a later typed ordering should be considered only after proof-output and search-order compatibility are pinned down.
- `mk_prim_enum_inst` runs `NormalizeEquations`, records/stores `DCPrimEnum`, and only then calls `BooleanSimplification` on the stored clause. Rust preserves the visible derivation ordering by applying Boolean simplification after `DCPrimEnum` proof metadata is attached.
- C's primitive-enumeration fresh-pattern helpers draw from the term bank's shared variable bank and assume its freshness counters already reflect live variables. Rust synchronizes used variable counts and the global fresh counter before this inference; later proof-state ownership should centralize fresh-variable seeding instead of doing it per inference.
<!-- END MANUAL REVIEW: c_source_docs -->
