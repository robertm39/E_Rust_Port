<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_signature

## Source Files

- [TERMS/cte_signature.h](../../../eprover/TERMS/cte_signature.h)
- [TERMS/cte_signature.c](../../../eprover/TERMS/cte_signature.c)

## Purpose

Definitions for dealing with signatures, i.e. data structures storing information about function symbols and their properties. the GNU Lesser General Public License. <1> Thu Sep 18 16:54:31 MET DST 1997

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FunConstCmpFunType`
- `FuncCell`
- `Func_p`
- `FunctionProperties`
- `PolyTypeCheckFun`
- `SigCell`
- `Sig_p`

### Macros And Constants

- `CTE_SIGNATURE`
- `DEFAULT_SIGNATURE_GROW`
- `DEFAULT_SIGNATURE_SIZE`
- `FuncDelProp(symb, prop)`
- `FuncIsAnyPropSet(symb, prop)`
- `FuncQueryProp(symb, prop)`
- `FuncSetProp(symb, prop)`
- `MULTI_ARITY_HACK`
- `SIG_CONS_CODE`
- `SIG_DB_LAMBDA_CODE`
- `SIG_FALSE_CODE`
- `SIG_FEATURE_ARITY_LIMIT`
- `SIG_ITE_CODE`
- `SIG_LET_CODE`
- `SIG_NAMED_LAMBDA_CODE`
- `SIG_NIL_CODE`
- `SIG_PHONY_APP_CODE`
- `SIG_TRUE_CODE`
- `SigCellAlloc()`
- `SigCellFree(junk)`
- `SigDefaultSort(sig)`
- `SigDelFuncProp(sig, symb, prop)`
- `SigExternalSymbols(sig)`
- `SigGetDepthFeatureOffset(sig, f)`
- `SigGetFCount(sig)`
- `SigGetNewDefCode(sig, arity)`
- `SigGetNewPredicateCode(sig, arity)`
- `SigGetNewSkolemCode(sig, arity)`
- `SigGetNewTypedDefCode(sig, args, num_args, ret_type)`
- `SigGetNewTypedSkolem(sig, args, num_args, ret_type)`
- `SigGetType(sig, f)`
- `SigInterpreteNumbers(sig)`
- `SigIsAnyFuncPropSet(sig, symb, prop)`
- `SigIsFunConst(sig, f_code)`
- `SigIsLogicalSymbol(sig, f_code)`
- `SigIsSimpleAnswerPred(sig, f_code)`
- `SigQueryFuncProp(sig, symb, prop)`
- `SigSetFuncProp(sig, symb, prop)`
- `TMP_LET_ID`

### Globals

- None found in the source scan.

### Exported Functions

- `((sig)->f_count-(sig)->internal_symbols) FunCode SigFindFCode(Sig_p sig, const char* name)`
- `FunCode SigGetNewFCode(Sig_p sig, int arity, char *prefix, long *counter, FunctionProperties props)`
- `FunCode SigGetOrNCode(Sig_p sig, int arity)`
- `FunCode SigGetOtherEqnCode(Sig_p sig, FunCode f_code)`
- `FunCode SigGetTypedApp(Sig_p sig, Type_p arg1, Type_p arg2, Type_p ret)`
- `FunCode SigInsertFOFOp(Sig_p sig, const char* name, int arity)`
- `FunCode SigInsertId(Sig_p sig, const char* name, int arity, bool special_id)`
- `FunCode SigInsertLetId(Sig_p sig, const char* name, Type_p type)`
- `FunCode SigParse(Scanner_p in, Sig_p sig, bool special_ids)`
- `FunCode SigParseKnownOperator(Scanner_p in, Sig_p sig)`
- `FunCode SigParseSymbolDeclaration(Scanner_p in, Sig_p sig, bool special_id)`
- `FunCode SigPopId(Sig_p sig)`
- `SigGetNewFCode((sig), (arity),"esk", &(sig)->skolem_count, FPSkolemSymbol) SigGetNewFCode((sig), (arity),"epred", &(sig)->newpred_count, FPDefPred) SigGetNewFCode((sig), (arity),"edef", &(sig)->newdef_count, FPDefFun) FunCode SigGetNewTypedFCode(Sig_p sig, char* prefix, Type_p* args, int num_args, long* counter, Type_p ret_type, FunctionProperties props)`
- `SigGetNewTypedFCode(sig, "esk", args, num_args, \ &(sig)->newpred_count, ret_type, FPDefPred) SigGetNewTypedFCode(sig, "edef", args, num_args, \ &(sig)->newdef_count, ret_type, FPDefFun) void SigDeclareType(Sig_p sig, FunCode f, Type_p type)`
- `Sig_p SigAlloc(TypeBank_p bank)`
- `Type_p TypeCheckArithBinop(struct sigcell *sig, struct termcell *t)`
- `Type_p TypeCheckArithConv(struct sigcell *sig, struct termcell *t)`
- `Type_p TypeCheckDistinct(struct sigcell *sig, struct termcell *t)`
- `Type_p TypeCheckEq(struct sigcell *sig, struct termcell *t)`
- `bool SigHasChoiceSym(Sig_p sig)`
- `bool SigHasUnimplementedInterpretedSymbols(Sig_p sig)`
- `bool SigIsFixedType(Sig_p sig, FunCode f_code)`
- `bool SigIsFunction(Sig_p sig, FunCode f_code)`
- `bool SigIsPolymorphic(Sig_p sig, FunCode f_code)`
- `bool SigIsPredicate(Sig_p sig, FunCode f_code)`
- `bool SigIsSpecial(Sig_p sig, FunCode f_code)`
- `bool SigQueryProp(Sig_p sig, FunCode f, FunctionProperties prop)`
- `bool SigSymbolUnifiesWithVar(Sig_p sig, FunCode f_code)`
- `int SigAddSymbolArities(Sig_p sig, PDArray_p distrib, bool predicates, long selection[])`
- `int SigCountAritySymbols(Sig_p sig, int arity, bool predicates)`
- `int SigCountSymbols(Sig_p sig, bool predicates)`
- `int SigFindMaxFunctionArity(Sig_p sig)`
- `int SigFindMaxPredicateArity(Sig_p sig)`
- `int SigFindMaxUsedArity(Sig_p sig)`
- `int SigFindMinFunctionArity(Sig_p sig)`
- `int SigFindMinPredicateArity(Sig_p sig)`
- `int SigGetAlphaRank(Sig_p sig, FunCode f_code)`
- `long SigBacktrack(Sig_p sig, FunCode f_count)`
- `long SigCollectSortConsts(Sig_p sig, Type_p type, PStack_p res)`
- `long SigFCodesCollectTypes(Sig_p sig, NumTree_p fcodes, PTree_p *types)`
- `static inline FunCode SigGetCNilCode(Sig_p sig)`
- `static inline FunCode SigGetEqnCode(Sig_p sig, bool positive)`
- `static inline FunCode SigGetOrCode(Sig_p sig)`
- `static inline char* SigFindName(Sig_p sig, FunCode f_code)`
- `static inline int SigFindArity(Sig_p sig, FunCode f_code)`
- `void SigDeclareFinalType(Sig_p sig, FunCode f, Type_p type)`
- `void SigDeclareIsFunction(Sig_p sig, FunCode f)`
- `void SigDeclareIsPredicate(Sig_p sig, FunCode f)`
- `void SigEnterLetScope(Sig_p sig, PStack_p type_decls)`
- `void SigExitLetScope(Sig_p sig)`
- `void SigFixType(Sig_p sig, FunCode f_code)`
- `void SigFree(Sig_p junk)`
- `void SigInsertInternalCodes(Sig_p sig)`
- `void SigParseTFFTypeDeclaration(Scanner_p in, Sig_p sig)`
- `void SigPrint(FILE* out, Sig_p sig)`
- `void SigPrintACStatus(FILE* out, Sig_p sig)`
- `void SigPrintAppEncodedDecls(FILE* out, Sig_p sig)`
- `void SigPrintSpecial(FILE* out, Sig_p sig)`
- `void SigPrintTypeDeclsTSTP(FILE* out, Sig_p sig)`
- `void SigPrintTypeDeclsTSTPSelective(FILE* out, Sig_p sig, NumTree_p *symbols)`
- `void SigPrintTypes(FILE* out, Sig_p sig)`
- `void SigSetAllSpecial(Sig_p sig, bool value)`
- `void SigSetPolymorphic(Sig_p sig, FunCode f_code, bool value)`
- `void SigUpdateFeatureOffset(Sig_p sig, FunCode f)`

## Implementation Notes

### Internal Functions

- `SigFindArity`
- `SigFindName`
- `SigGetCNilCode`
- `SigGetEqnCode`
- `SigGetFeatureOffset`
- `SigGetOrCode`
- `sig_compute_alpha_ranks`
- `sig_print_operator`

### Source-Level Behavior

- `SigFindArity`: Given signature and a function symbol code, return the arity of the symbol.
- `SigFindName`: Given signature and a function symbol code, return a pointer to the name. This pointer is only valid as long as the signature exists!
- `SigGetEqnCode`: Return the FunCode for $eq or $neq, create them if non-existant.
- `SigGetOrCode`: As above, for $or
- `SigGetCNilCode`: As above, for $cnil
- `SigGetFeatureOffset`: Return the feature offset of the symbol. This is arity limited by SIG_FEATURE_ARITY_LIMIT for function symbols, the same shifted up by SIG_FEATURE_ARITY_LIMIT for predicate symbols.
- `sig_print_operator`: Print a single operator
- `sig_compute_alpha_ranks`: For all symbols in sig compute the alpha-rank of the symbol.
- `SigAlloc`: Allocate a initialized signature cell. Also initializes a type table.
- `SigInitInternalCodes`: Put all the FOF operators as function symbols into sig. Sig should be empty, so that sig->internal symbols can be properly initialized. Note that this will be used for plain term signatures. It reuses some equivalent fields of signatures used for patterns, but morphs the f_codes into internal symbols.
- `SigFree`: Free signature.
- `SigFindFCode`: Return the index of the entry name in sig, or 0 if name is not in sig.
- `SigIsPredicate`: Returns true if the symbol is known to be a predicate
- `SigIsFunction`: Return the value of the Function field for a function symbol.
- `SigSetSpecial`: Set the value of the special field for a function symbol.
- `SigIsSpecial`: Return the value of the special field for a function symbol.
- `SigGetAlphaRank`: Given a signature and an function symbol code, return the symbols alpha-rank.
- `SigSetAllSpecial`: Set the special value of all symbols in sig.
- `SigInsertId`: Insert the symbol name with arity into the signature. Return the f_code assigned to the name or 0 if the same name has already been used with a different arity.
- `SigPopId`: Remove the last symbol from the signature. This should only be done when no structures use it - otherwise the behaviour is undefined. This also ignores the Let-Id-Stack. The function returns the old sig->f_count (equivalent to the removed identifier), or 0 if the signature is empty.
- `SigBacktrack`: Remove all symbols with f_codes > f_count from the signature. See SigPopId() for caveats. Returns the number of symbols popped.
- `SigInsertLetId`: Insert symbol with the given name and type whose name is of local character -- it will not be stored in f_index, and it will not be checked if the symbol already exists.
- `SigInsertFOFOp`: Insert a special function symbol used to encode a first-order operator.
- `SigPrint`: Print the signature in external representation, with comments showing internal structure.
- `SigPrintSpecial`: Print the external special symbols from sig.
- `SigPrintACStatus`: For each function symbol which is A, C, or AC, print its status as a comment.
- `SigParseKnownOperator`: Parse an operator, return its FunCode. Error, if operator is not in sig.
- `SigParseSymbolDeclaration`: Parse a single symbol declaration (f:3) and insert it into sig.
- `SigParse`: Parse a list of declarations into a signature.
- `SigFindMaxUsedArity`: Return the largest arity of any function symbol used in the signature.
- `SigFindMaxPredicateArity`: Return the largest arity of any predicate function symbol used in the signature.
- `SigFindMinPredicateArity`: Return the smallest arity of any predicate function symbol used in the signature.
- `SigFindMaxFunctionArity`: Return the largest arity of any real function symbol used in the signature.
- `SigFindMinFunctionArity`: Return the smallest arity of any real function symbol used in the signature.
- `SigCountAritySymbols`: Count number of symbols with a given arity. If predictates is true, count predicates, otherwise count function symbols.
- `SigCountSymbols`: Count number of symbols. If predictates is true, count predicates, otherwise count function symbols.
- `SigAddSymbolArities`: Count the occurences of symbols of a given arity (by adding one for each symbol to the corresponding entry in distrib). If predicates is true, count predicate symbols only, otherwise count function symbols only. Only looks at symbols where select[symbol] is true. Return maximal arity of relevant symbols.
- `SigCollectSortConsts`: Collect all constant symbols with the given sort onto res. Untyped symbols are assume to be type STIndividuals. Return number of constants found.
- `SigGetOrNCode`: Return FunCode for $orn, create them if non-existant.
- `SigGetOtherEqnCode`: If eqn_code is passed in, return neqn_code, and vice versa. Assumes FOF-initialized signature.
- `SigCetNewFCode`: Return an fcode for a new identifier (based on prefix) with the given arity and properties. The symbol is guaranteed to be new to sig.
- `SigGetNewTypedSkolem`: Return a new typed Skolem symbol based on the type of given free variables and return type.
- `SigGetNewTypedFCode`: Return a new typed symbol based on the type of given arguments and the return type.
- `SigDeclareType`: Declare the type of the given function. Will fail (and crash) if the type is already declared and is fixed.
- `SigDeclareFinalType`: Declare the type of the symbol, and fix it (cannot be changed)
- `SigDeclareIsFunction`: This symbol occurs in a function position (in an equation, as a subterm...).
- `SigDeclareIsPredicate`: This symbol occurs as a predicate, without ambiguity.
- `SigPrintTypes`: Prints symbols with their type to the given file descriptor.
- `SigPrintTypeDeclsTSTP`: Print TPTP-3 type declarations for all real symbols in sig.
- `SigPrintTypeDeclsTSTPSelective`: Print TPTP-3 type declarations for the symbols in sig that are also in symbols.
- `SigParseTFFTypeDeclaration`: Parses a type declaration, and update the signature if it is a symbol declaration.
- `SigHasUnimplementedInterpretedSymbols`: Return true if there are uninterpreted interpreted symbols in the signature (in which case the prover is incomplete).
- `SigFCodesCollectTypes`: Collect all types of symbols in fcodes (and their proper subtypes in the first-order sense) into types.
- `SigUpdateFeatureOffset`: Update the feature index of the symbol. Index is min(arity, SIG_FEATURE_ARITY_LIMIT-1) for predicate symbols, SIG_FEATURE_ARITY_LIMIT+min(arity, SIG_FEATURE_ARITY_LIMIT-1) for function symbols.
- `SigGetTypedApp`: Gets the symbol that corresponds term application of type (arg1 * arg2) > ret. This roughly corresponds to higher-order type (t1 -> t2) -> t1 -> t2, so the invariant is arg1 == t1 -> t2, arg2 == t1, ret = t2.
- `SigPrintAppEncodedDecls`: Prints type declarations that correspond to app-encoded terms.
- `SigSymbolUnifiesWithVar`: Checks whether f_code can be unified with a variable. In HO case variable unifies with any function code; in FOOL case variable unifies with $true, $false and non-predicate symbols.
- `SigEnterLetScope`: Enters a new scope in which the symbols from type decls will override the ones already present in the signature
- `SigExitLetScope`: Enters a new scope in which the symbols from type decls will override the ones already present in the signature
- `SigHasChoiceSym`: Count number of symbols with a given arity. If predictates is true, count predicates, otherwise count function symbols.

### Dependencies

- `"cte_signature.h"`
- `<clb_numtrees.h>`
- `<clb_pdarrays.h>`
- `<clb_properties.h>`
- `<clb_stringtrees.h>`
- `<cte_functypes.h>`
- `<cte_simpletypes.h>`
- `<cte_typebanks.h>`

### Compile-Time Conditions

- `CTE_SIGNATURE`
- `MULTI_ARITY_HACK`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_signature.h`, `TERMS/cte_signature.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 2763 lines, 71 scanned public declarations, 8 scanned internal function definitions, and 60 structured function-comment blocks.
- Function-symbol signature table. Arity, property bits, special symbols, and name interning underpin parsing and term construction.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `SigInsertInternalCodes` reserves fixed function codes for `$true`, `$false`, `$@_var`, named/DB lambdas, `$ite`, `$let`, and related built-ins before normal user symbols are parsed. Rust proof-state allocation and the remaining executable temporary parser banks now perform that reservation before inserting user symbols; otherwise an ordinary user predicate can receive `SIG_PHONY_APP_CODE` and be misclassified as a phony application.
- `SigSupportLists` is process-global in C and affects `SigAlloc`: when true, `$nil` and `$cons` are inserted as fixed internal symbols immediately after `$false`. Rust makes this state explicit on each `Signature` so term printers can distinguish real list-enabled signatures from ordinary user symbols named `$nil` or `$cons`.
- C `Signature` owns `ac_axioms` as a pointer stack of recognized AC clauses while the actual clauses remain owned elsewhere. Rust mirrors this as compact clause derivation refs on `Signature`; replace them with stable clause handles only when proof-state ownership can represent the same lifetime explicitly.

### Change Later

- Bare `Signature::new(TypeBank::new())` is useful in unit tests and low-level helpers, but executable/parser-facing banks need C's internal-code block. Once parser ownership is consolidated, prefer a named constructor for C-initialized parsing signatures so remaining temporary print/app-encode paths cannot bypass fixed-code reservation accidentally.
- If command-line parsing eventually allows list support to change after some signatures exist, compare C's global `SigSupportLists` timing against Rust's per-signature flag before exposing a higher-level API.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
