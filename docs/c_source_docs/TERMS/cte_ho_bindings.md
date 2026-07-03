<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_ho_bindings

## Source Files

- [TERMS/cte_ho_bindings.h](../../../eprover/TERMS/cte_ho_bindings.h)
- [TERMS/cte_ho_bindings.c](../../../eprover/TERMS/cte_ho_bindings.c)

## Purpose

Interface to the module which creates higher-order variable bindings. the GNU Lesser General Public License. <1> ma 25 okt 2021 10:35:21 CEST

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic, Petar Vukmirovic.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTE_HO_BINDINGS`
- `ELIM_MASK`
- `GET_ELIM(c)`
- `GET_IDENT(c)`
- `GET_IMIT(c)`
- `GET_PROJ(c)`
- `IDENT_MASK`
- `IMIT_MASK`
- `INC_ELIM(c)`
- `INC_IDENT(c)`
- `INC_IMIT(c)`
- `INC_PROJ(c)`
- `PROJ_MASK`

### Globals

- None found in the source scan.

### Exported Functions

- `StateTag_t ComputeNextBinding(Term_p var, Term_p rhs, StateTag_t state, Limits_t* limits, TB_p bank, Subst_p subst, HeuristicParms_p parms, bool* succ)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `build_imitation`: Builds imitation binding if rhs has a constant as the head. Otherwise returns NULL.
- `build_projection`: Projects onto argument idx if return type of variable at the head of flex returns the same type as the argument. Otherwise returns NULL. Inside the code idx is increased because flex is applied using PHONY_APP_CODE.
- `build_elim`: Eliminates argument idx. Always succeeds.
- `build_ident`: Builds identification binding. Must be called with both lhs and rhs top-level free variables. Then it returns.
- `build_trivial_ident`: Builds trivial identification binding. Must be called with both lhs and rhs top-level free variables. Then it returns.
- `SubstComputeFixpointMgu`: Assuming that flex is an (applied) variable and rhs an arbitrary term which are normalized and to which substitution is applied generate the next binding in an attempt to solve the problem flex =?= rhs. What the next binding is is determined by the value of 'state'. The last two bits of 'state' have special meaning (is the variable pair already processed) a...

### Dependencies

- `"cte_ho_bindings.h"`
- `"cte_lambda.h"`
- `"cte_pattern_match_mgu.h"`
- `<cte_ho_csu.h>`

### Compile-Time Conditions

- `CTE_HO_BINDINGS`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_ho_bindings.h`, `TERMS/cte_ho_bindings.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 546 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Interface to the module which creates higher-order variable bindings. the GNU Lesser General Public License. <1> ma 25 okt 2021 10:35:21 CEST
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- `cte_ho_bindings.c` stores four binding-generation counters in one `Limits_t` word: imitation in bits 0-5, projection in bits 6-11, identification in bits 12-17, and elimination in bits 18-23. Rust now ports the masks, field accessors, and C-shaped increment helpers in `src/terms/ho_bindings.rs`.
- Rust also ports the `build_imitation` binding constructor: it rejects variable/phony RHS terms with the C `NULL` shape, imitates monomorphic rigid constants directly, and imitates monomorphic rigid function symbols by applying them to fresh synthesized arguments built over the flex prefix DB variables.
- Rust also ports the `build_projection` binding constructor: it preserves C's one-step weak-head normalization gate, shallow rigid-head failure checks, non-function argument identity projection, and functional-argument branch that applies the projected DB variable to fresh synthesized arguments.
- Rust also ports the `build_elim` binding constructor: for an applied free variable it builds a fresh matrix variable over all visible DB arguments except the eliminated zero-based index, then closes the result under the full original visible argument prefix. Tests apply and beta-normalize the resulting binding for both two-argument elimination indexes.
- Rust also ports both identification constructors. `build_ident` allocates one fresh matrix variable over the concatenated left/right type prefixes and builds the C-ordered left/right target applications with fresh synthesized opposite-side prefix arguments; `build_trivial_ident` keeps the fallback that shares one fresh return-type matrix variable under both lambda prefixes.
- Rust now stages `ComputeNextBinding` as `compute_next_binding`: it preserves the C counter ranges for imitation, left/right projection, left/right elimination, and identification, mutates the Rust `Substitution`, advances the packed constraint state, and reports whether the substitution stack changed. The reusable Rust `CsuIterator` now consumes this dispatcher for queue/backtracking enumeration; proof-control call-site integration remains pending.

### Change-Later Observations

- The `INC_IMIT`/`INC_PROJ`/`INC_IDENT`/`INC_ELIM` macros increment their selected six-bit field without masking the incremented result back down to six bits, so overflow can carry into the next field. Rust preserves that arithmetic in the helper layer; a cleaned CSU binding API should use typed counters and explicit limit checks once reference behavior is covered.
- `build_imitation` refuses rigid symbols when `SigGetType` cannot return a concrete monomorphic type, leaving polymorphic imitation unimplemented. Keep that compatibility behavior for now; after full polymorphic higher-order unification coverage exists, this is a natural place to add typed instantiation instead of returning no binding.
- `build_projection` uses a deliberately shallow precheck: it compares only top-level DB heads or rigid function codes, skips weak-head normalization when the RHS is a top-level free variable, and lets some lambda-shaped cases proceed. Preserve this enumeration behavior until CSU traces prove a stronger precheck is compatible.
- `build_elim` always abstracts the full original argument prefix even though the fresh matrix variable is applied only to the retained arguments. Preserve this for CSU search parity; after complete trace coverage, consider whether a clearer binding representation could encode the dropped argument without reconstructing a lambda slot that is intentionally unused.
- `build_ident` uses a runtime-sized C stack array plus two `memcpy` calls to concatenate type prefixes before creating the matrix-variable type. Rust uses owned vectors for the same order; if the C side is revisited, replacing this variable-length stack allocation with an explicit heap/vector helper would improve portability and make the cross-prefix order easier to audit.
- `ComputeNextBinding` compares unsigned six-bit counter fields with signed heuristic limits; in C, negative limits convert to very large unsigned values and effectively allow the binding kind. Rust mirrors that behavior in the staged dispatcher. A future API should make unlimited-vs-zero explicit instead of relying on signed/unsigned conversion.
<!-- END MANUAL REVIEW: c_source_docs -->
