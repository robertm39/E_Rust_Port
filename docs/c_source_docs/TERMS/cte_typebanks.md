<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_typebanks

## Source Files

- [TERMS/cte_typebanks.h](../../../eprover/TERMS/cte_typebanks.h)
- [TERMS/cte_typebanks.c](../../../eprover/TERMS/cte_typebanks.c)

## Purpose

Declarations of functions needed for manipulating shared type objects. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TypeBank`
- `TypeBank_p`

### Macros And Constants

- `CTE_TYPEBANKS`
- `GetArity(node)`
- `GetNameId(node)`
- `NAME_NOT_FOUND`
- `REALLOC_STEP`
- `TYPEBANK_HASH_MASK`
- `TYPEBANK_SIZE`
- `TypeBankCellAlloc()`
- `TypeBankTypeIsUserDefined(bank, type)`
- `hash_type(type)`
- `type_a0hash(t)`
- `type_a1hash(t)`
- `type_aritynhash(t)`

### Globals

- None found in the source scan.

### Exported Functions

- `TypeBank_p TypeBankAlloc(void)`
- `TypeConsCode TypeBankDefineSimpleSort(TypeBank_p bank, const char* name)`
- `TypeConsCode TypeBankDefineTypeConstructor(TypeBank_p bank, const char* name, int arity)`
- `TypeConsCode TypeBankFindTCCode(TypeBank_p bank, const char* name)`
- `Type_p TypeBankInsertTypeShared(TypeBank_p bank, Type_p t)`
- `Type_p TypeBankParseType(Scanner_p in, TypeBank_p bank)`
- `Type_p TypeChangeReturnType(TypeBank_p bank, Type_p type, Type_p new_ret)`
- `const char* TypeBankFindTCName(TypeBank_p bank, TypeConsCode tc_code)`
- `int TypeBankFindTCArity(TypeBank_p bank, TypeConsCode tc_code)`
- `void TypeBankAppEncodeTypes(FILE* out, TypeBank_p tb, bool print_type_comment)`
- `void TypeBankFree(TypeBank_p junk)`
- `void TypeBankPrintSelectedSortDefs(FILE* out, TypeBank_p bank, PTree_p selector)`
- `void TypePrintTSTP(FILE* out, TypeBank_p bank, Type_p type)`

## Implementation Notes

### Internal Functions

- `bii_alloc`
- `ensure_not_kind`
- `force_arg_sharing`
- `type_arg_realloc`

### Source-Level Behavior

- `bii_alloc`: Allocates one cell of back_idx_info based on construction arguments.
- `cmp_types`: Wrapper for ad-hoc type comparison.
- `type_arg_realloc`: Reallocate new argument array if needed.
- `force_arg_sharing`: Make sure that arguments are shared.
- `ensure_not_kind`: Reports an error if argument is kind.
- `parse_single_type`: Parses one type and makes sure it is shared.
- `tree_free_fun`: Frees the type stored in the tree.
- `TypeBankAlloc`: Allocate TypeBank cell.
- `TypeBankFree`: Frees the whole typebank.
- `TypeBankInsertTypeShared`: Insert type t to type bank to make it shared. If the type t was not present in the bank, return new type and free the original type.
- `TypeBankDefineTypeConstructor`: Register type constructor with given name and arity.
- `TypeBankDefineSimpleSort`: Register simple sort with given name.
- `TypeBankFindTCCode`: Find type constructor code corresponding to given name. If the name is not found NAME_NOT_FOUND is returned.
- `TypeBankFindTCArity`: Return the arity of given type constructor. Behavior is undefined if type constructor does not exist (in debug mode error will be reported).
- `TypeBankFindTCName`: Return the name of given type constructor. Behavior is undefined if type constructor does not exist (in debug mode error will be reported).
- `TypeBankParseType`: Parses the TPTP FO type syntax (A1 * A2 * ... * An) > B or TPTP HO type syntax A1 > A2 > ... > An. Mixing of syntaxes is not allowed.
- `TypePrintTSTP`: Prints type in either FO or HO format, based on problemType status.
- `TypeBankPrintSelectedSortDefs`: Print type declarations for the types in selector that correspond to simple sorts. Selector is a PTree of Type_p keys.
- `TypeChangeReturnType`: generated type.
- `TypeBankAppEncodeTypes`: For each term application symbol according to type a > b print declaration app_ab_a_b : translation(a>b) * translation(a) > translation(b). If print_type_comment is true then the original, higher-order type will be printed as well.

### Dependencies

- `"cte_functypes.h"`
- `"cte_typebanks.h"`
- `<cio_scanner.h>`
- `<clb_objtrees.h>`
- `<clb_pdarrays.h>`
- `<clb_verbose.h>`
- `<cte_simpletypes.h>`

### Compile-Time Conditions

- `CTE_TYPEBANKS`
- `_count`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_typebanks.h`, `TERMS/cte_typebanks.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1036 lines, 18 scanned public declarations, 4 scanned internal function definitions, and 20 structured function-comment blocks.
- Type interning/banking. Type identity and sharing are expected by terms, signatures, and parser code.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
