<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_lambda

## Source Files

- [TERMS/cte_lambda.h](../../../eprover/TERMS/cte_lambda.h)
- [TERMS/cte_lambda.c](../../../eprover/TERMS/cte_lambda.c)

## Purpose

Functions that implement main operations of lambda calculus the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermNormalizer`

### Macros And Constants

- `CTE_LAMBDA`
- `DB_NOT_FOUND`

### Globals

- None found in the source scan.

### Exported Functions

- `TFormula_p NamedLambdaSNF(TB_p terms, TFormula_p t)`
- `TFormula_p NamedToDB(TB_p bank, TFormula_p lambda)`
- `TermNormalizer GetEtaNormalizer()`
- `Term_p AbstractVars(TB_p terms, Term_p matrix, PStack_p var_prefix)`
- `Term_p BetaNormalizeDB(TB_p bank, Term_p term)`
- `Term_p CloseWithDBVar(TB_p bank, Type_p ty, Term_p body)`
- `Term_p CloseWithTypePrefix(TB_p bank, Type_p* tys, long size, Term_p matrix)`
- `Term_p DecodeFormulasForCNF(TB_p bank, Term_p term)`
- `Term_p FlattenApps(TB_p bank, Term_p hd, Term_p* args, long num_args, Type_p res_type)`
- `Term_p LambdaEtaExpandDB(TB_p bank, Term_p term)`
- `Term_p LambdaEtaExpandDBTopLevel(TB_p bank, Term_p t)`
- `Term_p LambdaEtaReduceDB(TB_p bank, Term_p term)`
- `Term_p LambdaNormalizeDB(TB_p bank, Term_p term)`
- `Term_p PostCNFEncodeFormulas(TB_p bank, Term_p term)`
- `Term_p ShiftDB(TB_p bank, Term_p term, int shift_val)`
- `Term_p WHNF_deref(Term_p t)`
- `Term_p WHNF_step(TB_p bank, Term_p t)`
- `void SetEtaNormalizer(TermNormalizer)`

## Implementation Notes

### Internal Functions

- `ApplyTerms`
- `UnfoldLambda`

### Source-Level Behavior

- `ApplyTerms`: Fills var_stack with abstracted variables and returns the body of lambda For example, given ^[X]:(^[Y]:s), varstack = [Y, X] and s is returned
- `UnfoldLambda`: Fills var_stack with abstracted variables and returns the body of lambda. For example, given ^[X]:(^[Y]:s), varstack = [Y, X] and s is returned
- `drop_args`: Does eta-normalization in an optimized way: it does not do one lambda binder at the time (e.g. %x. (%y. g x y) -> %x. g x -> g ), but it does it in one go %xy. g x y -> g.
- `FlattenApps`: Apply additional arguments to hd assuming hd needs to be flattened.
- `flatten_and_make_shared`: Beta normalization and eta-reduction can result in a term that has PhonyApp symbol at head and a regular symbol as its first argument. This function performs the necessary flattening on the intermediary term created during either normalization procedure and makes sure that it is shared.
- `do_shift_db`: Performs the actual shifting.
- `replace_bound_vars`: 1. For DB vars that are bound, do nothing. 2a. For DB vars that are loosely bound with index 0 <= idx < total_bound, replace them with the corresponding term (shifted for depth) 2b. For other losely bound variables with index idx, shift them for total_bound - idx
- `find_min_db`: Find the loosely bound DB variable with the minimal index. Return DB_NOT_FOUND if no such variable exists.
- `reduce_eta_top_level`: Does one step of argument removal necessary for eta-reduction.
- `do_eta_expand_db`: Does eta-expansion on the lambda terms in the De Bruijn notation.
- `do_eta_reduce_db`: Does eta-normalization in an optimized way: it does not do one lambda binder at the time (e.g. %x. (%y. g x y) -> %x. g x -> g ), but it does it in one go %xy. g x y -> g.
- `do_beta_normalize_db`: Performs the actual beta-normalization.
- `do_named_to_db`: Performs the actual conversion. Tries to reduce recursion by doing multiple lambda steps at the same time. Recursion can be completely elimininated using two stacks: One with pairs (term_to_process, depth) and the other one which records the terms that have been decomposed. However, we go for recursion it is unlikely that we will have that extremely deep te...
- `encode_quantifiers_as_lambdas`: Encodes (![X,Y,Z]: body) into ! @ ^[X]: (! @ ^[Y]: ( ! @ ^[Z]: (body)))
- `replace_bvars`: Assuming free variables are bound to DB variables
- `CloseWithDBVar`: Given body of the lambda, create a term LAM.body where LAM is the abstraction constructor for DB var of type ty.
- `CloseWithTypePrefix`: Given an array of types [t1, t2, ..., tn] create a lambda term [X1:t1]: (... [Xn:tn]: (s))
- `SetEtaNormalizer`: Register a function that is going to be used for eta normalization.
- `GetEtaNormalizer`: Register a function that is going to be used for eta normalization.
- `NamedToDB`: Given *closed* lambda in the named representation, return the corresponding
- `ShiftDB`: Shifts all losely bound variables by *shift_val*
- `AbstractVars`: Abstract var_prefix over matrix. Variable at the top of the stack is the first one to abstract.
- `whnf_step_uncached`: Actaully compute whnf without considering cache.
- `WHNF_step`: Given a term of the form (%XYZ. body) x1 x2 x3 x4 ... Computes the term (body[X -> x1, Y -> x2; Z -> x3]) x4 ...
- `WHNF_deref`: Dereference and beta-normalize term only until the head of the term becomes known.
- `BetaNormalizeDB`: Normalizes de Bruijn encoded lambda terms
- `PostCNFEncodeFormulas`: Takes quantifiers encoded using a free variable into the ones which use DB.
- `DecodeFormulasForCNF`: Takes formulas that are in the form for proving and decodes them into the form necessary for CNF. Most importantly, atoms are encoded as $eq(term, $true) and lambda-encoded quantifiers are turned into variable-encoded ones.
- `LambdaEtaExpandDBTopLevel`: Do only one top-level step of eta expansion.
- `LambdaEtaReduceDB`: Performs eta-reduction on DB terms
- `LambdaEtaExpandDB`: Performs eta-expansion on DB terms
- `LambdaNormalizeDB`: Performs beta normalization, folowed by eta normalization.

### Dependencies

- `"cte_lambda.h"`
- `<ccl_derivation.h>`
- `<ccl_formula_wrapper.h>`
- `<ccl_inferencedoc.h>`
- `<ccl_pdtrees.h>`
- `<ccl_tformulae.h>`
- `<cte_subst.h>`
- `<cte_termbanks.h>`

### Compile-Time Conditions

- `CTE_LAMBDA`
- `NDEBUG`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_lambda.h`, `TERMS/cte_lambda.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1625 lines, 20 scanned public declarations, 2 scanned internal function definitions, and 32 structured function-comment blocks.
- Lambda calculus operations. De Bruijn shifting, beta normalization, eta reduction, and phony-application flattening are semantic details.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/terms/lambda.rs` now stages the DB-lambda helpers needed by higher-order argument pruning, equality-definition unfolding, and eta normalization: C-shaped `ApplyTerms`, `FreshVarWArgs` as the reusable `fresh_var_with_args`, `FlattenApps`, `flatten_and_make_shared`, `UnfoldLambda`, `NamedToDB`, `PostCNFEncodeFormulas`, `DecodeFormulasForCNF`, `drop_args`, `find_min_db`, `reduce_eta_top_level`, `LambdaEtaExpandDBTopLevel`, `LambdaEtaExpandDB`, `LambdaEtaReduceDB`, `SetEtaNormalizer`, `GetEtaNormalizer`, `LambdaNormalizeDB`, `CloseWithDBVar`, `CloseWithTypePrefix`, `AbstractVars`, `ShiftDB`, `WHNF_step`, explicit-bank `WHNF_deref`, and DB beta normalization.
- The staged beta normalizer handles phony applications headed by DB lambdas, consumed-argument substitution with DB-index shifting, recursive beta normalization under lambdas and ordinary top cells, and the C `BetaNormalizeDB` special case that unwraps `$eq(logical_symbol, $true)`.
- The represented lambda/DB normalization surface is complete through explicit-bank `WHNF_deref`; remaining formula CNF call-site integration belongs to formula ownership rather than hidden term ownership. Rust deliberately recomputes and shares weak-head results instead of storing C's LFHO-only cache pointers in every unified term cell. The semantic, layout, ordering, unification, and performance evidence for that decision is retained in [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).

### Change Later

- C `WHNF_step` writes temporary bindings into DB-variable cells and clears them manually after substitution. Rust avoids mutating DB variable cells by carrying an explicit binding vector indexed by DB index; keep this safer representation unless profiling or C trace comparison exposes a compatibility issue.
- C `AbstractVars` temporarily writes DB-variable bindings into free-variable cells and relies on `replace_fvars` to shift them under nested lambdas. Rust keeps the same stack order and DB-index shifting with an explicit free-variable binding map instead of mutating shared variable cells.
- C `NamedToDB` temporarily writes DB-variable bindings into named-lambda binder variables. Rust uses an explicit binder-depth map for the same De Bruijn indexes and restores shadowed entries structurally.
- C `PostCNFEncodeFormulas` temporarily writes DB-variable bindings into quantified free variables and shifts the matrix by the quantifier-prefix length before recursion. Rust preserves those index calculations with an explicit binding-depth map and structural restore.
- C `DecodeFormulasForCNF` routes every recursive result through `EncodePredicateAsEqn`, including encoding `$false` as `$true!=$true`. Rust mirrors that locally in `lambda.rs`; consider unifying it with the parser-side predicate encoder when full formula ownership gives that helper a stable public home.
- C `LambdaNormalizeDB` delegates eta behavior through a file-static normalizer function pointer. Rust mirrors the observable hook with a process-wide safe lock; if future strategy code wants scoped eta normalization, keep that scoping explicit instead of hiding additional mutable globals.
- C `flatten_and_make_shared` repairs intermediary phony applications whose head becomes an ordinary symbol after beta/eta work. Rust now ports that repair for the recursive eta paths; audit remaining beta-only rebuilding sites if a future caller uses them independently of `LambdaNormalizeDB`.
- The C comments contain several stale or copy-pasted descriptions around `ApplyTerms`, `UnfoldLambda`, and `GetEtaNormalizer`. Keep the source behavior, not the comments, as the compatibility reference.
<!-- END MANUAL REVIEW: c_source_docs -->
