<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_simpletypes

## Source Files

- [TERMS/cte_simpletypes.h](../../../eprover/TERMS/cte_simpletypes.h)
- [TERMS/cte_simpletypes.c](../../../eprover/TERMS/cte_simpletypes.c)

## Purpose

Stephan Schulz Implementation of simple types for the TSTP TFF (and THF) format. A complex ("arrow") type is an array [t1,...,tn, t], representing the type t1 -> ... -> tn -> t or (t1, ... tn) -> tn, depending on yout viewpoint. In particular, the last element in the array is the return sort.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Simon Cruanes (simon.cruanes@inria.fr), Simon Cruanes (simon.cruanes@inria.fr),  Petar Vukmirovic,

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TypeCell`
- `TypeConsCode`
- `TypeUniqueID`
- `Type_p`

### Macros And Constants

- `AllocArrowType(arity, args)`
- `AllocSimpleSort(code)`
- `ArrowTypeCons`
- `CTE_SIMPLETYPES`
- `GetRetType(t)`
- `INVALID_TYPE_UID`
- `NO_TYPE`
- `STBool`
- `STIndividuals`
- `STInteger`
- `STKind`
- `STPredefined`
- `STRational`
- `STReal`
- `SortIsInterpreted(sort)`
- `SortIsUserDefined(sort)`
- `TypeArgArrayAlloc(n)`
- `TypeArgArrayFree(junk, n)`
- `TypeCellAlloc()`
- `TypeCellFree(junk)`
- `TypeIsArrow(t)`
- `TypeIsBool(t)`
- `TypeIsIndividual(t)`
- `TypeIsKind(t)`
- `TypeIsPredicate(t)`
- `TypeIsTypeConstructor(t)`
- `VAR_ORDER(ty)`

### Globals

- None found in the source scan.

### Exported Functions

- `DStr_p TypeAppEncodedName(Type_p type)`
- `Type_p ArrowTypeFlattened(Type_p const* args, int args_num, Type_p ret)`
- `Type_p FlattenType(Type_p type)`
- `Type_p GetReturnSort(Type_p type)`
- `Type_p TypeCopy(Type_p orig)`
- `Type_p TypeDropFirstArg(Type_p ty)`
- `bool IsChoiceType(Type_p ty)`
- `bool TypeHasBool(Type_p t)`
- `int TypeGetMaxArity(Type_p t)`
- `int TypesCmp(Type_p t1, Type_p t2)`
- `void TypeFree(Type_p junk)`

## Implementation Notes

### Internal Functions

- `AllocArrowTypeCopyArgs`
- `TypeAlloc`
- `TypeGetOrder`
- `get_builtin_name`

### Source-Level Behavior

- `TypeAlloc`: Allocates new type cell.
- `AllocArrowTypeCopyArgs`: Allocates an arrow type where arguments of arrow are represented in a statically allocated array -- thus we need to dynamically allocate them and copy them in the dynamic array.
- `TypeGetOrder`: Calculates the order of the type.
- `is_flattened`: Checks if type t is represented as flattened that is it is either a unit type or it is a type such that the last argument is not arrow and all arguments are flattened.
- `arguments_flattened`: Checks if arguments of type t are flattened -- see is_flattened
- `get_builtin_name`: Returns the name of the built-in type in TPTP syntax.
- `TypeIsUntyped`: Return if the type does not contain any non-standard sorts, i.e. if it is a type that occurs in plain cnf/fof problems.
- `TypeCopy`: Creates a shallow copy of orig.
- `TypeTopFree`: Frees the type cell used by junk.
- `TypeFree`: Frees the type cell used by junk and the argument array.
- `TypesCmp`: Ad-hoc total order on types. Based on pointer values.
- `FlattenType`: Makes sure type is represented using flattened representation, i.e. the one where the last argument is not not arrow. IMPORTANT: Assumes all arguments are flattened. Return value is an unshared type.
- `GetReturnSort`: Returns the return type of function with the given type.
- `TypeAppEncodedName`: Encodes type as a string.
- `TypeGetMaxArity`: Given a type, determine what is the maximal arity of a function symbol.
- `TypeHasBool`: Does type have bool as an argument? Recursively checks in arguments
- `ArrowTypeFlattened`: Makes the flattened arrow type out of arguments and return type. Flattening refers to flattening out return type when arrow is constructed. If args_num is 0, returns return_type.
- `TypeDropFirstArg`: Drop the first argument of a type, creating a new, possibly unshared type. Assumes that type is arrow.
- `IsChoiceType`: Does the type correspond a monomorphized type of a choice symbol?

### Dependencies

- `"cio_scanner.h"`
- `"cte_simpletypes.h"`
- `<clb_ptrees.h>`

### Compile-Time Conditions

- `CTE_SIMPLETYPES`
- `STReal`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-07-17.

Source files reviewed: `TERMS/cte_simpletypes.h`, `TERMS/cte_simpletypes.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 724 lines, 15 scanned public declarations, 4 scanned internal function definitions, and 19 structured function-comment blocks.
- Simple type constructors/checking. Preserve built-in type symbols and arrow/product handling.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TypesCmp` orders first by constructor code, then arity, then the raw addresses of corresponding argument types through `PCmp`. C's own source notes that this causes clause-sorting differences; allocator address order and reuse are therefore process-local implementation details rather than reproducible cross-build values. Rust preserves the same rule with actual `Rc` allocation addresses, while `Rc::ptr_eq` preserves type identity. On 64-bit targets, both `Type` and `Option<Type>` remain one pointer wide.
- Shared types remain live in their `TypeBank`, so the address used for identity and ordering is stable for the bank's lifetime. Rust's reference counting replaces manual `TypeFree`/`TypeTopFree` ownership without changing the pointer-identity predicates used by `IsChoiceType`, signature/type-bank sharing, term comparison, or higher-order ordering.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
