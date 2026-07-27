<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termbanks

## Source Files

- [TERMS/cte_termbanks.h](../../../eprover/TERMS/cte_termbanks.h)
- [TERMS/cte_termbanks.c](../../../eprover/TERMS/cte_termbanks.c)

## Purpose

Definitions for term banks - i.e. shared representations of terms as defined in cte_terms.h. Uses the same struct, but adds administrative stuff and functionality for sharing. There are two sets of funktions for the manangment of term trees: Funktions operating only on the top cell, and functions descending the term structure. Top level functions implement a conventional splay

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TBCell`
- `TB_p`
- `TermMapper`

### Macros And Constants

- `CACHE_THRESHOLD`
- `CTE_TERMBANKS`
- `MAYBE_NORMALIZE_APP_VAR(t)`
- `TBCellAlloc()`
- `TBCellFree(junk)`
- `TBCellIdent(term)`
- `TBGCDeregisterClauseSet(terms, set)`
- `TBGCDeregisterFormulaSet(terms, set)`
- `TBGCRegisterClauseSet(terms, set)`
- `TBGCRegisterFormulaSet(terms, set)`
- `TBNonVarTermNodes(bank)`
- `TBPrintTermFull(out, bank, term)`
- `TBSortTable(tb)`
- `TBStorage(bank)`
- `TBTermCellIsMarked(bank, term)`
- `TBTermIsConjectureGroundTerm(term)`
- `TBTermIsGround(t)`
- `TBTermIsSubterm(super, term)`
- `TBTermIsTypeTerm(term)`
- `TBTermIsXTypeTerm(term)`
- `TermIsFalseTerm(term)`
- `TermIsTrueTerm(term)`

### Globals

- `extern bool TBPrintDetails`
- `extern bool TBPrintInternalInfo`
- `extern bool TBPrintTermsFlat`

### Exported Functions

- `TB_p TBAlloc(Sig_p sig)`
- `TermPrint((out), (term), (bank)->sig, DEREF_NEVER) void TBPrintTerm(FILE* out, TB_p bank, Term_p term, bool fullterms)`
- `Term_p NormalizePatternAppVar(TB_p bank, Term_p s)`
- `Term_p ParseIte(Scanner_p in, TB_p bank)`
- `Term_p ParseLet(Scanner_p in, TB_p bank)`
- `Term_p TBAllocNewSkolem(TB_p bank, PStack_p variables, Type_p type)`
- `Term_p TBCreateConstTerm(TB_p bank, FunCode const)`
- `Term_p TBCreateMinTerm(TB_p bank, FunCode min_const)`
- `Term_p TBFind(TB_p bank, Term_p term)`
- `Term_p TBFindRepr(TB_p bank, Term_p term)`
- `Term_p TBGetFirstConstTerm(TB_p bank, Type_p sort)`
- `Term_p TBGetFreqConstTerm(TB_p terms, Type_p sort, long* conj_dist_array, long* dist_array, FunConstCmpFunType is_better)`
- `Term_p TBInsertDisjoint(TB_p bank, Term_p term)`
- `Term_p TBInsertIgnoreVar(TB_p bank, Term_p term, DerefType deref)`
- `Term_p TBInsertInstantiated(TB_p bank, Term_p term)`
- `Term_p TBInsertInstantiatedDeref(TB_p bank, Term_p term, DerefType deref)`
- `Term_p TBInsertInstantiatedFO(TB_p bank, Term_p term)`
- `Term_p TBInsertInstantiatedHO(TB_p bank, Term_p term, bool follow)`
- `Term_p TBInsertNoProps(TB_p bank, Term_p term, DerefType deref)`
- `Term_p TBInsertNoPropsCached(TB_p bank, Term_p term, DerefType deref)`
- `Term_p TBInsertOpt(TB_p bank, Term_p term, DerefType deref)`
- `Term_p TBInsertRepl(TB_p bank, Term_p term, DerefType deref, Term_p old, Term_p repl)`
- `Term_p TBInsertReplPlain(TB_p bank, Term_p term, Term_p old, Term_p repl)`
- `Term_p TBTermParseReal(Scanner_p in, TB_p bank, bool check_symb_prop)`
- `Term_p TBTermParseSimple(Scanner_p in, TB_p bank)`
- `Term_p TBTermTopInsert(TB_p bank, Term_p t)`
- `Term_p TermApplyArg(TypeBank_p tb, Term_p s, Term_p arg)`
- `Term_p TermMap(TB_p bank, Term_p t, TermMapper f)`
- `long TBGCSweep(TB_p bank)`
- `long TBTermCollectSubterms(Term_p term, PStack_p collector)`
- `long TBTermDelPropCount(Term_p term, TermProperties prop)`
- `long TBTermNodes(TB_p bank)`
- `long TBTermSetPropCount(Term_p term, TermProperties prop)`
- `void TBFree(TB_p junk)`
- `void TBPrintBankInOrder(FILE* out, TB_p bank)`
- `void TBPrintBankTerms(FILE* out, TB_p bank)`
- `void TBPrintTermCompact(FILE* out, TB_p bank, Term_p term)`
- `void TBRefDelProp(TB_p bank, TermRef ref, TermProperties prop)`
- `void TBRefSetProp(TB_p bank, TermRef ref, TermProperties prop)`
- `void TBVarSetStoreFree(TB_p bank)`

## Implementation Notes

### Internal Functions

- `TBRawTermParse`
- `TBRequestDBVar`
- `TBTermParse`
- `choose_subterm_parse_fun`
- `make_let`
- `normalize_boolean_terms`
- `parse_let_sym_def`
- `parse_let_typedecl`
- `tb_parse_cons_list`
- `tb_print_dag`
- `tb_subterm_parse`
- `tb_term_parse_arglist`
- `tb_termtop_insert`

### Source-Level Behavior

- `tb_print_dag`: Print the terms as a dag in the order of insertion.
- `tb_termtop_insert`: Insert a term into the term bank for which the subterms are already in the term bank. Will reuse or destroy the top cell!
- `tb_parse_cons_list`: Parse a LOP list into an (shared) internal $cons list.
- `parse_let_typedecl`: Parses a single type declaration that constitutes of the first part of a let term. For each parsed symbol, on type_decl it stores symbol name (DStr), fresh symbol ID (regardless of whether the symbol is already in the signature), and symbol type ACHTUNG: Dynamically allocated DStr is put on the stack.
- `parse_let_definition`: Parses a single type declaration that constitutes of the first part of a let term. For each parsed symbol, on type_decl it stores symbol name (DStr), fresh symbol ID (regardless of whether the symbol is already in the signature), and symbol type ACHTUNG: Dynamically allocated DStr is put on the stack.
- `make_let`: Bulids the variable-arity let term.
- `tb_subterm_parse`: Parse a subterm, i.e. a term which cannot start with a predicate symbol.
- `choose_subterm_parse_fun`: If the argument to be parsed should be of boolean type, parse the argument as a formula. Otherwise, parse it as before.
- `normalize_boolean_terms`: If term_ref points to an equation of type X=true that appears under context, replace this equation by X.
- `tb_term_parse_arglist`: Parse a list of terms (comma-separated and enclosed in brackets) into an array of (shared) term pointers. See TermParseArgList() in cte_terms.c for more.
- `TBAlloc`: Allocate an empty, initialized termbank.
- `TBFree`: Free a term bank (if the signature alread has been extracted). Voids all pointers to terms in the bank!
- `TBVarSetStoreFree`: Free and reset the VarSetStore in bank.
- `TBTermNodes`: Return the number of term nodes (variables and non-variables) in the term bank.
- `TBInsert`: Insert the term into the termbank. The original term will remain untouched. The routine returns a pointer to a new, shared term of the same structure. TermProperties are masked with bank->prop_mask.
- `TBInsertInstantiatedDeref`: Insert the term, following the bindings of the variables according to DerefType.
- `TBInsertIgnoreVar`: As TBInsert, but does instead of using variables from the term bank, uses the ones already present in the temr. TermProperties are masked with bank->prop_mask.
- `TBInsertNoProps`: As TBInsert, but will set all properties of the new term to 0 first.
- `TBInsertNoPropsCached`: As TBInsert, but will set all properties of the new term to 0 first. Also, use a cache so that work is not repeated for the same terms.
- `TBInsertRepl`: As TBInsertNoProps, but when old is encountered as a subterm (regardless of instantiation), replace it with uninstantiated repl (which _must_ be in bank).
- `TBInsertReplPlain`: As TBInsertReplPlain, but terms are not instantiated.
- `TBInsertInstantiatedFO`: Insert a term into the termbank under the assumption that it is a right side of a rule (or equation) composed of terms from bank, and (possibly) instantiated with terms from bank - i.e. don't insert terms that are bound to variables and ground terms, but assume that they are in the term bank. Properties in newly created nodes are deleted.
- `TBInsertInstantiatedHO`: Differs from TBInsertInstantiatedFO by inserting every binding in the termbank. The reason is that bindings might be unshared terms, so we need to make sure we share them.
- `TBInsertInstantiated`: Wrapper that chooses which function to call based on the problem type.
- `TBInsertOpt`: Insert term into bank under the assumption that it it already is in the bank (except possibly for variables appearing as bindings). This allows us to just return term for ground terms.
- `TBInsertDisjoint`: Create a copy of (uninstantiated) term with disjoint variables. This assumes that all variables in term are odd or even, the returned copy will have variable ids shifted by -1.
- `TBTermTopInsert`: See tb_termtop_insert, for export without hurting inlining capabilities.
- `TBAllocNewSkolem`: Create a news Skolem term (or definition atom) with the given variables in the term bank and return the pointer to it.
- `TBFind`: Find a term in the term cell bank and return it.
- `TBPrintBankInOrder`: Print the DAG in the order of ascending entry_no.
- `TBPrintTermCompact`: Print a term bank term. Introduce abbreviations for all subterms encountered. Subterms with TPOutputFlag are not printed, but are assumed to be known. Does _not_ follow bindings (they are temporary and as such make little sense in the term bank context)
- `TBPrintTerm`: Print a term from a term bank either in compact form (with abbreviations) or as a conventional term.
- `TBPrintBankTerms`: Print the terms inserted into the term bank with abbreviations.
- `TBTermParseReal`: Parse a term from the given scanner object directly into the termbank. Supports abbreviations. This function will _not_ set the TPTopPos property on top terms while parsing. It will or will not check and set symbol properties (function symbol, predicate symbol), depending on the check_symb_prop parameter.
- `TBTermParseSimple`: Parses terms without giving any special semantics to symbols. Input variant of TermPrintSimple().
- `TBRefSetProp`: Make ref point to a term of the same structure as *ref, but with properties prop set. Properties do not work for variables!
- `TBRefDelProp`: Make ref point to a term of the same structure as *ref, but with properties prop deleted. If the term is a variable, do nothing!
- `TBTermDelPropCount`: Delete properties prop in term, return number of term cells with this property. Does assume that all subterms of a term without this property also do not carry it!
- `TBTermSetPropCount`: Set properties prop in term, return number of term cells changed. Does assume that all subterms of a term with this property already not carry it!
- `TBGCMarkTerm`: Mark a term as used for the garbage collector.
- `TBGCSweep`: Sweep the term bank and free all unmarked term cells. bank->true_term will be marked automatically. Returns the number of term cells recovered.
- `TBCreateConstTerm`: Create constant term for a given symbol.
- `TBCreateMinTerm`: If bank->min_term exists, return it. Otherwise create and return it.
- `TBTermCollectSubterms`: Collect all subterms of term onto collector. Assumes that TPOpFlag is set if and only if the term is already in the collection. Returns the number of new terms found.
- `TBFindRepr`: Find the representation of a term from another (or none) bank in this bank.
- `TBGetFirstConstTerm`: Return a constant term with the first constant of the proper sort in sig.
- `TBGetFreqConstTerm`: Find the best (according to is_better) constant of the give sort, and return a shared term with this constant. If no suitable constant exists, returns NULL. conj_dist_array contains number of occurrences for each symbol in conjecture clauses, dist_array the same for all clauses.
- `TermMap`: Applies the function f to term t to obtain t'. If t' != t, it continues mapping t'. Else, it recursively applies f to arguments of t. Result term is guaranteed to be shared. Term mapper must also return shared term of the same type as the original one. IMPORTANT: If f returns NULL this signifies that recursion should stop and the term is unaltered: this is...
- `ParseLet`: Parses let according to the TPTP description: http://ceur-ws.org/Vol-2162/paper-07.pdf. If top_level is true, let appears at the formula level and its body must be Bool. Otherwise, its body is parsed as a non-Bool.
- `ParseIte`: Parses ite according to the TPTP description: http://ceur-ws.org/Vol-2162/paper-07.pdf. If top_level is true, ite appears at the formula level and its body must be Bool. Otherwise, its body is parsed as a non-Bool.
- `NormalizePatternAppVar`: Tries to normalize applied variable so that all of its arguments are (?!?)

### Dependencies

- `"cte_termbanks.h"`
- `"cte_typecheck.h"`
- `<ccl_tformulae.h>`
- `<cio_basicparser.h>`
- `<clb_numtrees.h>`
- `<cte_dbvars.h>`
- `<cte_garbage_coll.h>`
- `<cte_lambda.h>`
- `<cte_termcellstore.h>`
- `<cte_typebanks.h>`
- `<cte_varsets.h>`

### Compile-Time Conditions

- `CTE_TERMBANKS`
- `ENABLE_LFHO`
- `NDEBUG`
- `NEVER_DEFINED`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for shared-subterm traversal ownership on 2026-07-21 and duplicate top-shell reuse on 2026-07-25.

Source files reviewed: `TERMS/cte_termbanks.h`, `TERMS/cte_termbanks.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 2820 lines, 47 scanned public declarations, 13 scanned internal function definitions, and 51 structured function-comment blocks.
- Shared term bank. Term identity, sharing, and garbage-collection interaction are central performance contracts.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- The recursive insertion family (`TBInsert`, `TBInsertIgnoreVar`, `TBInsertNoProps`, cached no-props, `TBInsertOpt`, `TBInsertRepl`) and `TBInsertInstantiatedDeref` combine one-step dereferencing with the LFHO applied-variable prefix rule: after expanding a bound applied free variable, arguments from the binding prefix keep `DEREF_NEVER` while later original arguments keep the caller's deref mode. Rust mirrors the prefix result through bank-local no-cache expansion; allocating WHNF normalization is exposed separately through the explicit-bank lambda API. Omitting C's hidden term owner/cache is the completed measured decision in [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Compatibility Notes

- `TBTermParseReal` rejects argument lists after integer, rational, floating-point, or object tokens while the corresponding `signature->distinct_props` bit remains set, and reports them through `AktTokenError` with the source, line, column, current `(` token, and the `--free-numbers` or `--free-objects` hint. Rust's checked bank parser now preserves that full diagnostic context for clause/equation parsing as well as the acceptance effect of clearing the distinct-property bits.
- `TBTermParseReal` treats `[` as a term start when `SigSupportLists` is true and delegates to `tb_parse_cons_list`, which recursively parses elements through `TBTermParseReal` and then inserts the resulting `$nil`/`$cons` spine into the term bank.
- `TBTermParseReal` special-cases `$ite` and `$let` before ordinary operator parsing. Rust mirrors checked `$ite` and scoped Boolean `$let` through the term-bank-local TSTP formula subset, including let-local signature shadowing and encoded equality definitions, and now preserves fixed non-predicate atom, compound, scoped-variable, newly introduced default-individual zero-arity, and typed-context fresh compound branches in non-Boolean `$ite` terms plus fixed non-Boolean `$let` definitions and bodies.
- `TBTermParseReal` parses non-Boolean typed arguments through `tb_subterm_parse`, so fixed predicates used as function arguments are rejected, unfixed predicate-typed symbols are passed through `SigDeclareIsFunction`/`TypeInferSort`, and ordinary function symbols have their type fixed. Rust mirrors this path for checked non-Boolean term arguments, and now routes Boolean-typed arguments through a term-bank-local `TFormulaTSTPParse` subset for truth constants, atoms, equality/disequality, negation, first-order quantifiers, named lambda formula operands and function-valued lambda term operands, typed higher-order formula application with `@` including parenthesized logical heads, `$ite`, scoped Boolean `$let`, associative `&`/`|` chains with conjunction binding before disjunction, and one following non-associative FOF connective.
- C callers can reach higher-order term application through the `TFormulaTSTPParse` application sub-parser after the process-global problem type has been set by surrounding THF input. Rust exposes that term-valued application path as a narrow public `parse_tstp_application_term` entry point for executable target parsers while keeping the ordinary simple and checked term parsers separate.
- `TBTermParseSimple` is intentionally looser: despite parsing the same token classes and inserting the same function-property bits, it does not reject distinct numeric/object symbols with argument lists. Rust keeps the simple parser permissive and uses the checked variant only for clause paths that correspond to C `TBTermParse`.
- `TBPrintTerm` reaches conventional term printing through `TermPrint(..., DEREF_NEVER)`. Rust keeps the compact/DAG bank printers in this module and adds explicit term-bank writers for the currently ported first-order and higher-order conventional term surfaces, including first-order FOOL/`$let` rendering and higher-order FOOL/lambda rendering, rather than reading process-global `problemType`.
- `TBPrintBankInOrder` is represented by entry-number-ordered Rust writers, including an explicit `TBPrintInternalInfo` property-comment mode used by the ported `term2dag` executable.
- `TBStorage` estimates one dynamic term-cell block per shared non-variable term plus one term-pointer unit per stored argument under C's constant-memory branch. Rust now exposes this same estimate from the term-bank owner for proof-state cleanup-limit accounting.
- `TBTermCollectSubterms` marks each new shared node with `TPOpFlag`, pushes its pointer, and recurses directly over `term->args`. Rust now matches that ownership shape with one immutable argument-slice borrow per visited parent and clones only the handle retained by the owned collector. Exact proof, constrained-resource, full-matrix, and 2.2366% Callgrind evidence are retained in [`experiment 176`](../../../experiments/2026-07-21-176-borrowed-subterm-collection/FINDINGS.md).
- `TBTermTopInsert` propagates child properties and computes variable count, function count, and weight by traversing `term->args` directly. Rust now holds one scoped immutable argument-slice borrow for the same loop instead of allocating a vector of cloned child handles; the borrow ends before the computed metadata is stored. Exact proof, constrained-resource, full-matrix, and 1.1212% Callgrind evidence are retained in [`experiment 177`](../../../experiments/2026-07-21-177-borrowed-insert-metadata/FINDINGS.md).
- `TBTermTopInsert` treats an applied free variable that `NormalizePatternAppVar` accepts as one variable for `v_count`, `f_count`, and weight, while still setting `TPHasAppVar` and leaving `TPHasNonPatternVar` clear. Rust now runs the shared eta-reducing normalizer at the same post-sharing metadata boundary, accepts visible DB arguments with the C-shaped duplicate check, and retains the original applied cell while applying the same cached measures. The loose-DB eta-redex regression and full validation are retained in [experiment 335](../../../experiments/2026-07-25-034-eta-pattern-metadata/FINDINGS.md).
- `TBGCMarkTerm` marks C's LFHO term cache because that cache owns otherwise reachable shared terms. Rust has no per-term LFHO cache, so there is no corresponding hidden GC root; ordinary arguments, rewrite replacements, proof owners, and term-bank roots cover every represented owner. This is a consequence of the explicit-bank decision, not omitted marking.
- `tb_termtop_insert` expresses its top-cell, lambda/application, shared-child, inferred-type, computed-weight, groundness, and post-insertion lookup invariants as C assertions. The normal upstream build defines `NDEBUG`, so Rust keeps the corresponding checks as `debug_assert!`; this avoids scanning and cloning arguments plus repeating a term-store lookup on millions of release insertions while retaining the checks in debug builds.
- `tb_termtop_insert` destroys a rejected duplicate through `TBCellFree`, whose C size-class allocator can reuse the top cell. Rust now recovers the same reuse opportunity with a bounded bank-local pool for uniquely owned rejected shells of arity zero through two. `Rc::get_mut` proves exclusivity before every full reset; canonical/shared cells and externally retained duplicates are never recycled, and the cap of eight cells per eligible arity bounds idle storage. Exact proof, safety regressions, a 1.9169% matched Callgrind reduction, and comprehensive compatibility evidence are retained in [`experiment 311`](../../../experiments/2026-07-25-010-reuse-duplicate-top-shells/FINDINGS.md).
- `TBInsertOpt` assumes ground terms are already shared and, in higher-order `DEREF_ALWAYS` mode, C reaches dereferencing through `WHNF_deref`. Rust currently keeps the no-WHNF insertion path explicit but shares unshared ground terms exposed by applied-variable dereferencing so the bank invariant still holds for lambda-headed applications.

### Change Later

- A `TBCell` is not a copyable value even though much of its payload looks pointer-owned: its `term_store` roots refer to `TermCell` objects whose `lson`/`rson` fields are the store's intrusive splay links. Two independently mutated banks therefore cannot alias the same non-variable term cells. Rust now makes both `TermBank` and its `TermCellStore` explicitly non-`Clone`; containing proof, learning, and evaluation owners are non-cloneable as a consequence. Parser capability probes allocate detached empty banks with signature snapshots, while proof-object renderers borrow the canonical bank immutably and use print-only literal/clause views that cannot relink the store or change term classification. A later C cleanup should likewise make this single-owner contract explicit, or move index links into store-owned nodes with stable term handles.
- The C `$let` parser creates let-local function codes that remain in `sig->f_info`, temporarily rewires `f_index` for the body, and uses a variable-length stack array for definition parameters. Rust preserves the visible scoped-name behavior with explicit safe vectors and scope restoration for currently supported Boolean and fixed non-Boolean definitions/bodies, but future signature cleanup may need a deliberate policy for unreachable local symbols once full FOOL/formula preprocessing owns their lifetimes.
- `choose_subterm_parse_fun` sends Boolean-typed term arguments into the formula parser, while THF lambda bodies can be term-valued or formula-valued depending on context. Rust preserves that mixed parser boundary with a lambda-body-specific plain-term path and explicit Boolean operand checks for logical connectives; a future full THF parser should split term and formula grammar entry points instead of relying on the C formula-atom fallback.
- `ParseIte` routes all arms through `TFormulaTSTPParse`, whose infix-equation parser can return a bare fixed non-predicate term rather than a formula literal and whose predicate-atom encoding can temporarily classify a fresh branch symbol or application as `atom = $true`. Rust mirrors the C-visible term context by recovering newly introduced untyped branch atoms when a sibling branch or surrounding non-Boolean argument sort forces a term sort; a full FOOL parser should separate term-valued and formula-valued branch parsing instead of depending on this encoded-atom fallback.
- The full and simple term-bank parsers share much of their syntax shape but enforce different distinct-symbol and argument-position policies. Future Rust parser APIs should keep that difference explicit until full `TBTermParseReal` parity, remaining term-valued FOOL `$ite`/`$let` ambiguity beyond expected-sort/default-individual branch recovery, and caller audits prove a single parser entry point is safe.
- Production C compiles every `tb_termtop_insert` invariant check out, including child-sharing validation and the post-insertion `TBFind`. A cleaned term-bank API should validate externally constructed cells at a narrow checked boundary and keep the internal insertion path explicitly trusted, rather than choosing between unchecked release state and repeated hot-path validation.
- `tb_subterm_parse` calls `SigDeclareIsFunction` for any unfixed predicate-typed symbol used as a non-Boolean argument, but `SigDeclareIsFunction` only changes exact `$o` types; arrow predicates are fixed while retaining a Boolean return sort. Rust preserves this through the signature helper. Revisit once full typed formula parsing can show whether this ambiguity is observable beyond compatibility.
- `tb_parse_cons_list` reconstructs shared lists through a stack of tail-placeholder cells rather than inserting a straightforward head-to-tail spine. Rust builds and inserts a valid `$nil`-terminated `$cons` chain from the parsed element vector; compare against C reference traces before deciding whether any placeholder-stack edge behavior is observable.
- `TBInsertInstantiatedDeref` immediately returns the original term for `DEREF_NEVER`, so C callers such as Lambda-order KBO rely on already-banked term metadata when no dereferencing is requested. Rust preserves that shape; a later cleaned API may want a named "prepare for lambda comparison" step that guarantees sharing/metadata even when dereferencing is disabled.
- The C `TBInsertOpt` ground fast path relies on a global `problemType` branch to call `WHNF_deref` for higher-order `DEREF_ALWAYS`; that mixes sharing, dereferencing, and weak-head normalization policy in one API. Rust should keep the current explicit sharing fix for unshared ground expansions, then revisit whether `insert_opt` itself should own HO WHNF once term-bank/session ownership can represent the C global cleanly.
- Conventional term printing still spans `cte_termfunc` global type/list switches, cache-backed dereferencing/WHNF paths, and the C split where first-order `$let` is pretty-printed but higher-order `$let` remains ordinary application. Keep the term-bank writer APIs explicit until those compatibility surfaces and executable output paths are audited, then decide whether a single problem-type-dispatched wrapper is useful.
- `TBPrintBankInOrder` allocates and frees a temporary `NumTree` every time it needs sorted entry-number output. Rust sorts collected bank terms directly; preserve output order, but do not cargo-cult the temporary tree unless reference/performance data gives a reason.
- The `TermTopCompare` header commentary says term properties participate in the sharing key, but the implementation compares function code, arity, type, and argument pointers without comparing `properties`. Rust follows the implementation. The C comment should be corrected later, and any future property-sensitive sharing design must first audit callers that mutate flags on already shared terms.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
