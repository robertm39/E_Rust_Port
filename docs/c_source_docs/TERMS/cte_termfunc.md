<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termfunc

## Source Files

- [TERMS/cte_termfunc.h](../../../eprover/TERMS/cte_termfunc.h)
- [TERMS/cte_termfunc.c](../../../eprover/TERMS/cte_termfunc.c)

## Purpose

Most of the user-level functionality for unshared terms. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `VarNormStyle`

### Macros And Constants

- `CTE_TERMFUNC`
- `PRINT_AT`
- `PRINT_HO_PAREN(out, ch)`
- `TERM_APPLY_APP_VAR_MULT(w, t, p)`
- `TRIM_THRESHOLD`
- `TermAddSymbolDistribution(term, dist_array)`
- `TermDefaultWeight(term)`
- `TermIsGround(term)`
- `TermPrint(out, term, sig, deref)`
- `TermPrintDbg(out, term, sig, deref)`
- `TermStandardWeight(term)`
- `TermStartToken`
- `TermWeight(term, vweight, fweight)`

### Globals

- `extern bool TermPrintLists`
- `extern bool TermPrintTypes`

### Exported Functions

- `(FuncSymbStartToken|OpenSquare|Mult): \ (FuncSymbStartToken|Mult)) void VarPrint(FILE* out, FunCode var)`
- `(TermIsShared(term)? \ TBTermIsGround((term)): \ TermIsGroundCompute((term))) FunCode TermFindMaxVarCode(Term_p term)`
- `(fputc((ch), (out))) : 0) Term_p TermCopyUnifyVars(VarBank_p vars, Term_p term)`
- `FunCode TermSigInsert(Sig_p sig, const char* name, int arity, bool special_id, FuncSymbType type)`
- `FuncSymbType TermParseOperator(Scanner_p in, DStr_p id)`
- `TermAddSymbolDistributionLimited((term),(dist_array), LONG_MAX) void TermAddSymbolDistributionLimited(Term_p term, long *dist_array, long limit)`
- `TermPrintHO(out, term, sig, deref) : TermPrintFO(out, term, sig, deref)) TermPrintDbgHO(out, term, sig, deref) : TermPrintFO(out, term, sig, deref)) void TermPrintSimple(FILE* out, Term_p term, Sig_p sig)`
- `Term_p TermAppEncode(Term_p orig, Sig_p sig)`
- `Term_p TermCheckConsistency(Term_p term, DerefType deref)`
- `Term_p TermCopy(Term_p source, VarBank_p vars, DBVarBank_p dbvars, DerefType deref)`
- `Term_p TermCopyKeepVars(Term_p source, DerefType deref)`
- `Term_p TermCopyNormalizeVars(VarBank_p vars, Term_p term, VarNormStyle var_norm)`
- `Term_p TermCopyNormalizeVarsAlpha(VarBank_p vars, Term_p term)`
- `Term_p TermCopyRenameVars(NumTree_p* renaming, Term_p term)`
- `Term_p TermCreatePrefix(Term_p orig, int up_to)`
- `Term_p TermParse(Scanner_p in, Sig_p sig, VarBank_p vars)`
- `Term_p TermParseArgList(Scanner_p in, Sig_p sig, VarBank_p vars)`
- `Term_p TermTrimImplications(Sig_p sig, Term_p f)`
- `Term_p TrimImplication(Sig_p sig, Term_p f)`
- `bool TermArrayNoDuplicates(Term_p* args, long size)`
- `bool TermFindIteSubterm(Term_p t, PStack_p pos)`
- `bool TermHasFCode(Term_p term, FunCode f)`
- `bool TermHasUnboundVariables(Term_p term)`
- `bool TermIsDBClosed(Term_p term)`
- `bool TermIsDefTerm(Term_p term, int min_arity)`
- `bool TermIsFlat(Term_p t)`
- `bool TermIsGroundCompute(Term_p term)`
- `bool TermIsSubterm(Term_p super, Term_p test, DerefType deref)`
- `bool TermIsSubtermDeref(Term_p super, Term_p test, DerefType deref_super, DerefType deref_test)`
- `bool TermIsUntyped(Term_p t)`
- `bool TermStructEqual(Term_p t1, Term_p t2)`
- `bool TermStructEqualDeref(Term_p t1, Term_p t2, DerefType deref_1, DerefType deref_2)`
- `bool TermStructEqualNoDeref(Term_p t1, Term_p t2)`
- `bool TermStructPrefixEqual(Term_p l, Term_p r, DerefType d_l, DerefType d_r, int remaining, Sig_p sig)`
- `int TermComputeOrder(Sig_p sig, Term_p term)`
- `long TermAddFunOcc(Term_p term, PDArray_p f_occur, PStack_p res_stack)`
- `long TermCollectFCodes(Term_p term, NumTree_p *tree)`
- `long TermCollectGroundTerms(Term_p term, PTree_p *result, bool all_subtems)`
- `long TermCollectPropVariables(Term_p term, PTree_p *tree, TermProperties prop)`
- `long TermCollectVariables(Term_p term, PTree_p *tree)`
- `long TermDAGWeight(Term_p term, long fweight, long vweight, long dup_weight, bool new_term)`
- `long TermDepth(Term_p term)`
- `long TermLexCompare(Term_p t1, Term_p t2)`
- `long TermLinearize(PStack_p stack, Term_p term)`
- `long TermNonLinearWeight(Term_p term, long vlweight, long vweight, long fweight)`
- `long TermStructWeightCompare(Term_p t1, Term_p t2)`
- `long TermSymTypeWeight(Term_p term, long vweight, long fweight, long cweight, long pweight)`
- `long TermWeightCompute(Term_p term, long vweight, long fweight)`
- `long VarBankCheckBindings(FILE* out, VarBank_p bank, Sig_p sig)`
- `static inline Term_p TermEquivCellAlloc(Term_p source, VarBank_p vars)`
- `void TermAddSymbolDistExist(Term_p term, long *dist_array, PStack_p exists)`
- `void TermAddSymbolFeatures(Term_p term, PStack_p mod_stack, long depth, long *feature_array, long offset)`
- `void TermAddSymbolFeaturesLimited(Term_p term, long depth, long *freq_array, long* depth_array, long limit)`
- `void TermAddTypeDistribution(Term_p term, Sig_p sig, long* type_arr)`
- `void TermAssertSameSort(Sig_p sig, Term_p t1, Term_p t2)`
- `void TermComputeFunctionRanks(Term_p term, long *rank_array, long *count)`
- `void TermFOOLPrint(FILE* out, Sig_p sig, Term_p form)`
- `void TermPrettyPrintSimple(FILE* out, Term_p term, Sig_p sig, int level)`
- `void TermPrintArgList(FILE* out, Term_p *args, int arity, Sig_p sig, DerefType deref)`
- `void TermPrintArgListRaw(FILE* out, Term_p *args, int arity, Sig_p sig, DerefType deref)`
- `void TermPrintDbgHO(FILE* out, Term_p term, Sig_p sig, DerefType deref)`
- `void TermPrintDbgVarBinds(Sig_p sig, Term_p t)`
- `void TermPrintFO(FILE* out, Term_p term, Sig_p sig, DerefType deref)`
- `void TermPrintHO(FILE* out, Term_p term, Sig_p sig, DerefType deref)`
- `void TermPrintSExpr(FILE* out, Term_p term, Sig_p sig)`

## Implementation Notes

### Internal Functions

- `GetHeadType`
- `TermEquivCellAlloc`
- `parse_cons_list`
- `print_cons_list`
- `term_check_consistency_rek`

### Source-Level Behavior

- `GetHeadType`: Returns the type of the head term symbol.
- `TermEquivCellAlloc`: Return a pointer to a unshared termcell equivalent to source. If source is a variable, get the cell from the varbank, otherwise copy the cell via TermTopCopy().
- `print_cons_list`: Print a list of $cons'ed terms, terminated with $nil. Abort on not well-formed lists (no cons pairs!).
- `parse_cons_list`: Parse a LOP list into an internal $cons list.
- `term_check_consistency_rek`: Traverse a tree and check if any one term cell occurs more than once on any branch (which would make the term cyclic). Return the first inconsistency found or NULL.
- `discard_last`: Returns the term where the last argument is left out. Assumes that there is at least one argument!
- `create_var_renaming_de_bruin`: Traverse a term and create alpha-normalizing variable renaming.
- `print_let`: Prints let term
- `do_is_db_closed`: Does the actual closeness check.
- `do_ho_print`: Inner function
- `do_fool_print`: Inner function
- `VarPrint`: Print a variable with FunCode var out.
- `TermPrintFO`: Print a FO term to the given stream.
- `TermPrintHO`: Print a HO term to the given stream. If PRINT_AT is defined terms will be delimited by @, otherwise " ".
- `TermPrintDbgHO`: Prints the term as is, with no pretty printing of interpreted symbols.
- `TermPrintArgList`: Print an argument list (i.e. an array with at least one term element) to the given stream.
- `TermPrintSimple`: Print a FO term without giving any special semantics to symbols -- basically prints the serialized syntax tree.
- `TermPrintSExpr`: Prints the (uninstantiated) term as an s-expression, with symbols/formula as naked ans possible.
- `TermIsFlat`: Return true if the term has no nested subterms.
- `TermPrettyPrintSimple`: Print a FO term without giving any special semantics to symbols -- basically prints the serialized syntax tree in a nicely formatted manner.
- `TermParseOperator`: Parse an operator (i.e. an optional $, followed by an identifier), store the representation into id and determine the type.using the following rules: - If it starts with a $, it's a TermIdentInterpreted (LOP global variables are treated as interpreted constants). - If it is a PosInt, it is a TermIdentNumber - If its a String, it is a TermIdentObject - If it...
- `TermSigInsert`: Thin wrapper around SigInsertId that also sets corresponding properties for different identifier types.
- `TermParse`: Parse a term from the given scanner object into the internal term representation.
- `TermParseArgList`: Parse a list of terms (comma-separated and enclosed in brackets) into an array of term pointers. Return the actual term containing the terms parsed. Note: The array has to have exactly the right size, as it will be handled by Size[Malloc|Free] for efficiency reasons and may otherwise lead to a memory leak. This leads to some complexity... If the arglist is...
- `TermCopy`: Return a copy of a given term. The new term will be unshared (except, of coure, for the variables) even if the original term was shared. Variable cells will be allocated from the VarBank given to the function.
- `TermCopyKeepVars`: Return a copy of a given term. The new term will be unshared (except, of coure, for the variables) even if the original term was shared. Variable cells will not be copied. Note that printing such a term might be confusing, since two variables with the same f_code may indeed be different!
- `TermStructEqual`: Return true if the two terms have the same structure. Follows bindings.
- `TermStructEqualNoDeref`: Return true if the two terms have the same structures. Ignores bindings.
- `TermStructEqualDeref`: Return true if the two terms have the same structures. Dereference both terms as designated by deref_1, deref_2.
- `TermStructPrefixEqual`: Return true if the two terms have the same structures except there are trailing arguments in r. Dereference both terms as designated by deref_1, deref_2.
- `TermStructWeightCompare`: Compare two terms based on just structural criteria: First compare standard-weight, then compare top-symbol arity, then compare subterms lexicographically. $true is always minimal.
- `TermLexCompare`: Compare two terms lexicographically by f_codes.
- `TermIsSubterm`: Return true if test is a subterm to super.
- `TermIsSubtermDeref`: Return true if test is a subterm to super. Uses TermStructEqualDeref() for equal test. NB: Deref is not changed since the function is not used.
- `TermWeightCompute`: Compute the weight of a term, counting variables as vweight and function symbols as fweight.
- `TermFsumWeight`: Return a weighted sum of the function symbols weights (and variable weights) in the term.
- `TermNonLinearWeight`: Compute the weight of a term, counting variables that occur for the first time as vlweight, varaibes that reoccur as vweight, and function symbols as fweight.
- `TermSymTypeWeight`: Compute the weight of a term, giving different weight to variables, constants, function symbols and predicates.
- `TermDepth`: Return the depth of a term.
- `TermIsDefTerm`: Return true if t is of the form f(X1...Xn) with n>=arity.
- `TermHasFCode`: Return true if f occurs in term, false otherwise. NB: DeBruijn variables are ignored.
- `TermHasUnboundVariables`: Return if the term contains unbound variables. Does not follow bindings.
- `TermIsGroundCompute`: Return if the term contains no variables. Does not follow bindings.
- `TermFindMaxVarCode`: Return largest (absolute, i.e. largest negative) f_code of any variable in term.
- `TermFindIteSubterm`: Returns true if it finds an $ite subterm in t. pos is the position corresponding to this subterm if it is found, empty otherwise.
- `VarBankCheckBindings`: For all variables in bank, check if they are bound. If sig!=0, print the variable and binding as a comment, otherwise just print variable number. Return number of bound variables.
- `TermAddSymbolDistributionLimited`: Count occurences of function symbols with f_code<limit in dist_array. Terms are not dereferenced!
- `TermAddTypeDistribution`: Count occurences of types of symbols in term and store them in type_arr
- `TermAddSymbolDistribExist`: Compute the distribution of symbols in term. Push all occuring symbols onto exists (once ;-).
- `TermAddSymbolFeaturesLimited`: Add function symbol frequencies and deepest depth of a function symbol to the two arrays. This is an extension of the function above, this one does the extendet task in a single term traversal. Note that function symbols >=limit are counted in array[0] for both depth and frequency.
- `TermAddSymbolFeatures`: Add function symbol frequencies and deepest depth of a function symbol to the array. offset should be 0 for positive literals, 2 for negative literals. Thus, the 4 features for a given f are stored at indices follows: - 4*f_code: |C^+|_f - 4*f_code+1: d_f(C^+) - 4*f_code+2: |C^-|_f - 4*f_code+3: d_f(C^-)
- `TermComputeFunctionRanks`: Assign an occurrence rank to each symbol in term.
- `TermCollectPropVariables`: Insert all variables with properties prop in term into tree. Return number of new variables.
- `TermCollectVariables`: Insert all variables in term into tree. Return number of new variables.
- `TermCollectFCodes`: Insert all f_codes in term into tree. Return number of new f_codes found
- `TermCollectGroundTerms`: Add non-constant (non-boolean) ground subterms of term to result. If all_subterm is false, only add maximal (in the subterm relation sense) terms, otherwise add all non-constant ground terms. Returns number of terms newly added.
- `TermAddFunOcc`: Add all new occurences of function symbol to res_stack and mark them as no-longer-new in f_occur. Return number of new function symbols added.
- `TermArrayNoDuplicates`: Checks if there are no duplicates in the
- `TermLinearize`: Put all subterms of term onto PStack in left-right preorder. Note that for an empty stack, that makes the index of s on the stack equal to its TermCPos. Returns number of subterms.
- `TermCheckConsistency`: Traverse a tree and check if any one term cell occurs more than once on any branch (which would make the term cyclic). Return the first inconsistency found or NULL.
- `TermAppEncode`: App-encodes the term.
- `TermCreatePrefix`: Create a prefix containing arg_num arguments of original term orig. If orig was an applied variable and arg_num is 0, return the shared variable that is the first argument. NB: In case caller needs proper prefix, returned term will not be shared (unless it is a variable head of applied variable)!
- `TermFOOLPrint`: Print a formula using only signature (not bank). Prints equations as infix.
- `TermCopyNormalizeVarsAlpha`: Create an alpha-normalized term copy.
- `TermCopyUnifyVars`: Create a term copy with all the variables unified (to X0).
- `TermCopyRenameVars`: Create a term copy using the specified variable normalization.
- `TermDAGWeight`: Compute the DAG weight of a term. More concretely: For each occurrence of an already considered subterm, count dup_weigth. For all new termcells count fweight for function sybmbols and vweight for variables. The new_term parameter indicates if the term shall be considered individually, or if this is a continuation of a previous computation which already mig...
- `TermIsDBClosed`: Checks if term has no leaky variables.
- `TermApplyArg`: Applies one term to the other. Performs rudimentary typechecking. Term is UNSHARED.
- `TermComputeOrder`: Computes the maximal order of the symbols that appear in the term.
- `TermPrintVarBinds`: Prints all the variables and their bindings from the term t
- `TermTrimImplications`: Consider only the conclusion part of the implication for considering the symbols in SinE.

### Dependencies

- `"clb_plocalstacks.h"`
- `"cte_termfunc.h"`
- `"cte_typecheck.h"`
- `<ccl_tformulae.h>`
- `<clb_numtrees.h>`
- `<cte_dbvars.h>`
- `<cte_lambda.h>`
- `<cte_pattern_match_mgu.h>`
- `<cte_termpos.h>`
- `<cte_termvars.h>`

### Compile-Time Conditions

- `CTE_TERMFUNC`
- `ENABLE_LFHO`
- `PRINT_AT`
- `STRICT_TPTP`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for recursive argument-access profiling on 2026-07-13 and shared structural-weight comparison on 2026-07-25.

Source files reviewed: `TERMS/cte_termfunc.h`, `TERMS/cte_termfunc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 3804 lines, 69 scanned public declarations, 5 scanned internal function definitions, and 72 structured function-comment blocks.
- Most of the user-level functionality for unshared terms. the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Compatibility Notes

- `TermWeight` and `TermStandardWeight` are macros that use the cached `v_count`, `f_count`, and `weight` fields for shared terms, while unshared terms fall back to recursive `TermWeightCompute`. Rust mirrors that split and debug-checks bank-owned metadata against recursive computation so the C performance contract does not hide stale shared-term metadata.
- `TermWeightCompute` counts a normalized pattern applied free variable as one variable rather than counting its application spine. Rust matches that result for bank-classified pattern applications; ordinary non-pattern applied variables retain structural recursive weight.
- `TermPrintFO` uses bracket notation only when both `SigSupportLists` and `TermPrintLists` are true and the dereferenced root is `$nil` or `$cons`. `print_cons_list` recursively prints elements with `TermPrint`, but validates the tail as a raw proper `$cons` chain ending in `$nil`; a bound tail variable does not make an improper list proper for this printer.
- `TermParse` treats `[` as a term start only when `SigSupportLists` is true, building a `$cons` spine with a final `$nil` cell. It recursively calls `TermParse` for elements, so nested list literals and ordinary terms share the same variable/type-annotation behavior.
- `TermPrintDbg` is a macro: in first-order mode it is just `TermPrintFO`, while higher-order mode dispatches to `TermPrintDbgHO`, which prints space-separated application without interpreted-symbol pretty-printing. Rust exposes this as a problem-type-explicit term-bank debug writer so callers do not need to read process-global `problemType`; the explicit deref-aware variant mirrors ordinary binding traversal and the LFHO applied-variable `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule. C's incidental cache mutation is not observable output and is deliberately omitted under the explicit-bank/no-per-term-cache decision in [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).
- `TermPrintHO` delegates to `do_ho_print`, which routes logical Boolean terms and lambdas through `do_fool_print`, prints non-formula higher-order applications with ` @ ` separators, skips the hidden phony-application head when printing arguments, spells DB variables using the current binder depth, handles `$ite`, and applies the LFHO applied-variable `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule. Rust exposes this through an explicit term-bank writer, including the FOOL equation/connective/quantifier/lambda surface and the higher-order parenthesis macro behavior.
- `TermPrintTypes` is a process-global formatting switch consulted by the full conventional term printers after the term body and after every recursively printed argument, yielding suffixes such as `f(a:$i):$i`. The compact term-bank abbreviation printer does not consult this switch. Rust now keeps typed output as an explicit term/equation print option for supported full-term clause output; avoid reintroducing a global switch except as an executable-compatibility shim.
- `TermParse` rejects an argument list after an integer or object token when the signature still treats that token class as distinct, using diagnostics that point to `--free-numbers` or `--free-objects`. Unlike `TBTermParseReal`, this unshared parser does not have separate rational/float branches. Rust keeps the unshared check separate from the term-bank checked parser so both C surfaces remain testable.
- `TermCheckConsistency` applies the LFHO applied-variable `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule while walking a single branch for repeated term cells and writes a start banner, one dot per visited dereferenced node, and an end banner directly to stdout. Rust mirrors the consistency check with pointer-identity branch tracking and no-cache, no-WHNF applied-variable expansion, and now exposes an explicit output-aware compatibility wrapper for the C progress text while keeping the pure helper output-free.
- `TermCopy` and `TermCopyKeepVars` use the LFHO applied-variable `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule when copying through one-step dereferenced applied variables. Rust mirrors the copied term and keeps later normalization behind explicit-bank APIs; it intentionally does not copy C's hidden source-owner pointer.
- `TermStructEqualDeref`, `TermStructPrefixEqual`, and `TermIsSubterm` use the LFHO applied-variable `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule when descending through one-step dereferenced applied variables. Rust mirrors this through result-equivalent no-cache read-only expansion; allocating beta/WHNF normalization remains an explicit-bank operation.
- `TermStructWeightCompare` walks raw shared-term argument and type pointers after cached-weight equality. Rust now matches that path with a private non-owning cursor only when both roots are shared; unshared roots retain the safe owned implementation. Shared descendants remain owned and structurally/type immutable, mutable argument guards reject shared terms, and the cursor invokes no mutation or callbacks. Exact comparator-owner work falls 37.64%, matched LUSK6 work falls 1.47%, and 128 alternating native pairs improve paired mean wall/CPU time by 1.0314%/1.0312% with 107 wins. The contract, equivalence tests, and comprehensive validation are retained in [`experiment 317`](../../../experiments/2026-07-25-016-borrowed-struct-weight-compare/FINDINGS.md).
- `TermTrimImplications` skips leading quantified formula wrappers, follows only the right-hand side of a `$impl` chain, and returns the consequent only after at least 10 implications; otherwise it returns the original formula pointer. Rust mirrors this threshold and right-spine-only behavior with shared `Term` handles for SInE symbol collection.

### Change Later

- The C weight macros hide the requirement that every shared term have immutable, valid cached counts and standard weight, while `TermWeightCompute` calls `NormalizePatternAppVar`, whose result depends on term-bank and eta-normalization state. Rust preserves the observable fast path and pattern result; a future typed shared-term constructor should make metadata validity explicit and separate nominal weight inspection from hidden normalization side effects.
- C exposes child terms through the raw `args` array and arity field, which makes recursive comparisons allocation-free but also lets unrelated code mutate children without a constructor or cache invalidation boundary. Rust uses a measured private cursor for the shared structural-weight comparator and keeps safe read-only argument slices elsewhere. A cleaned C/Rust term API should provide an explicit immutable shared child view and confine child mutation to metadata-maintaining builders.
- `TermPrintLists` and `SigSupportLists` are process-global formatting/allocation switches. Rust currently keeps list support explicit on `Signature` and routes bracket notation through term-bank printing; once executable option handling is complete, decide whether a global compatibility shim is needed or whether explicit per-signature state is preferable.
- `parse_cons_list` allocates the eventual `$nil` cell through the same arity-2 default-cell path used for `$cons` placeholders. Rust uses a zero-arity `$nil` for valid shared and unshared term shapes; compare this against reference behavior before deciding whether the allocation artifact needs to be externally preserved.
- The C parser surfaces differ: unshared `TermParse` checks integer/object argument lists, full `TBTermParseReal` also checks rational/float argument lists, and `TBTermParseSimple` does not check distinct-token argument lists at all. Keep the distinction for compatibility, but future internal APIs should name the checked/full parser path explicitly so callers do not accidentally choose the looser simple parser.
- `TermCheckConsistency` mixes debug progress output with a structural consistency query. Rust keeps that behavior behind an explicit output-aware wrapper; after compatibility is secured, prefer the output-free helper for ordinary structure checks and reserve the wrapper for C-compatible diagnostics.
- `TermIsSubtermDeref` comments say the deref is not changed because the function is not used; unlike `TermIsSubterm`, it does not apply `DEREF_LIMIT`/`CONVERT_DEREF` while descending. Keep this compatibility shape for now, but a cleaned helper should either use the same prefix rule as `TermIsSubterm` or be removed if no real caller needs it.
- Conventional C term printing mixes term and formula responsibilities: `TermPrintFO` special-cases `$let` before logical-symbol printing, while `do_ho_print` does not call `print_let`, so a higher-order `$let` falls through the ordinary application surface. Rust preserves this split for compatibility; after reference output traces are stable, consider separating term, formula, and let rendering behind clearer problem-type-specific APIs.
- `TermTrimImplications` uses a hard-coded threshold of 10 and only trims right-nested implication chains after leading quantifiers. Preserve this for SInE compatibility; after reference axiom-selection traces are stable, consider replacing the magic threshold with an explicit formula-normalization policy.
- `TermAppEncode` constructs and type-infers only the temporary prefix while relying on the original term, its directly applied argument, and the final result to already carry inferred types. Rust mirrors those assertions; a cleaned API should take an explicit type-checking context and return structured errors instead of depending on hidden preconditions.
- `TermApplyArg` performs type checking by pointer-identical type handles, grows ordinary non-variable/non-lambda heads in place, but switches variables and lambdas to a phony application cell before inserting the dropped-arrow result type into the supplied type bank. Preserve this constructor split for compatibility; a future application builder should make type-bank mutation and phony-application policy explicit.
- `TermComputeOrder` takes a signature argument that the C body does not use, starts from the root type's order, and skips only the lambda binder slot while scanning subterm types. Rust keeps those details; a later helper can drop the unused signature parameter and name the lambda-body-only traversal if no compatibility caller observes the old signature.
- `TermCollectVariables` allocates and frees a generic default `PStack` on every call even though this traversal is hot and its live depth is often small. Rust now avoids per-node argument-vector copies but still allocates one traversal `Vec` per call. A later C/Rust cleanup could reuse caller-owned search scratch or use a small inline stack while retaining direct argument-array access, left-to-right pushes, and cached-ground pruning.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
