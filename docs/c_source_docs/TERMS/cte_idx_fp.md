<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_idx_fp

## Source Files

- [TERMS/cte_idx_fp.h](../../../eprover/TERMS/cte_idx_fp.h)
- [TERMS/cte_idx_fp.c](../../../eprover/TERMS/cte_idx_fp.c)

## Purpose

Compute a fingerprint of a term suitable for fingerprint indexing. A fingerprint is a vector of individual samples for positions p, where the result is t|p->f_code if p is a position in t, BELOW_VAR (=LONG_MIN) if p<=q, t|q=Xn, 0 otherwise.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FPIndexFunction`
- `IndexFP_p`

### Macros And Constants

- `ANY_VAR`
- `BELOW_VAR`
- `CTE_IDX_FP`
- `MAX_PM_INDEX_NAME_LEN`
- `NOT_IN_TERM`
- `TermFPFlexSample(term, seq)`

### Globals

- `extern char* FPIndexNames[]`

### Exported Functions

- `FPIndexFunction GetFPIndexFunction(char* name)`
- `FunCode TermFPFlexSample(Term_p term, IntOrP* *seq)`
- `FunCode TermFPFlexSampleFO(Term_p term, IntOrP* *seq)`
- `FunCode TermFPFlexSampleHO(Term_p term, IntOrP* *seq)`
- `FunCode TermFPSample(Term_p term, ...)`
- `FunCode TermFPSampleFO(Term_p term, va_list ap)`
- `FunCode TermFPSampleHO(Term_p term, va_list ap)`
- `IndexFP_p IndexDTCreate(Term_p t)`
- `IndexFP_p IndexFP0Create(Term_p t)`
- `IndexFP_p IndexFP1Create(Term_p t)`
- `IndexFP_p IndexFP2Create(Term_p t)`
- `IndexFP_p IndexFP3DCreate(Term_p t)`
- `IndexFP_p IndexFP3DFlexCreate(Term_p t)`
- `IndexFP_p IndexFP3WCreate(Term_p t)`
- `IndexFP_p IndexFP4DCreate(Term_p t)`
- `IndexFP_p IndexFP4MCreate(Term_p t)`
- `IndexFP_p IndexFP4WCreate(Term_p t)`
- `IndexFP_p IndexFP4X2_2Create(Term_p t)`
- `IndexFP_p IndexFP5MCreate(Term_p t)`
- `IndexFP_p IndexFP6MCreate(Term_p t)`
- `IndexFP_p IndexFP7Create(Term_p t)`
- `IndexFP_p IndexFP7MCreate(Term_p t)`
- `IndexFP_p IndexFPFlexCreate(Term_p t, PStack_p pos, int len)`
- `IndexFP_p IndexFPfpCreate(Term_p t)`
- `void IndexFPFree(IndexFP_p junk)`
- `void IndexFPPrint(FILE* out, IndexFP_p fp)`

## Implementation Notes

### Internal Functions

- `push_fcodes`

### Source-Level Behavior

- `push_fcodes`: Push the f_codes of the term (in depth first, LR order) onto the stack.
- `TermFPSampleFO`: Sample the term at the position described by the optional arguments (encoding a (-1)-terminated position.
- `TermFPSampleHO`: For details see TermFPSampleFO(). It differs by supporting prefix matching/unification, where terms can have trailing arguments.
- `TermFPSample`: Based on problem type, chooses appropriate fingerprinting function.
- `TermFPFlexSample`: Sample the term at the position described by the array at pos. Update pos to point behind the end of the (-1)-terminated position.
- `TermFPFlexSampleHO`: Similar to TermFPFlexSample(), but supports HO fingerprinting.
- `IndexFP0Create`: Create a dummy fingerprint structure.
- `IndexFPfpCreate`: Create a fingerprint structure using an abstraction to just avoid function/predicate unifications/matches.
- `IndexFP1Create`: Create a fingerprint structure representing top symbol hashing.
- `IndexFP2Create`: Create a fingerprint structure representing sampling at epsilon, 0.
- `IndexFP3DCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 0.0.
- `IndexFP3WCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1.
- `IndexFP4DCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 0.0, 0.0.0
- `IndexFP4WCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1, 2
- `IndexFP4MCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1, 0.0
- `IndexFP5MCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1, 2, 0.0
- `IndexFP6MCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1, 2, 0.0, 0.1
- `IndexFP7Create`: Create a fingerprint structure with samples at positions epsilon, 0, 1, 0.0, 0.1, 1.0, 1.1 (using E's internal numbering).
- `IndexFP7MCreate`: Create a fingerprint structure representing sampling at epsilon, 0, 1, 2, 3, 0.0, 0.1
- `IndexFP4X2_2Create`: Create a fingerprint structure with samples at positions as specified below.
- `IndexFPFlexCreate`: Create a fingerprint of len elments, with the positions in pos.
- `IndexFP3DFlexCreate`: Testfunction, equivalent to IndexFP3DCreate()
- `IndexDTCreate`: Create a fingerprint that samples t at all its positions (in depths-first LR order) and no others. Building an FP-Tree with these samples will not build an FP-Index, but a (non-perfect) discrimination tree. This means that retrieval will require special code, it cannot use simple FP-Index retrieval.
- `IndexFPFree`: Free an IndexFP data-structure (i.e. a self-describing FunCode array).
- `GetFPIndexFunction`: Given a name, return the corresponding index function, or NULL.
- `IndexFPPrint`: Print a term fingerprint.

### Dependencies

- `"cte_idx_fp.h"`
- `"cte_simpletypes.h"`
- `<cte_termtypes.h>`
- `<stdarg.h>`

### Compile-Time Conditions

- `CTE_IDX_FP`
- `ENABLE_LFHO`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_idx_fp.h`, `TERMS/cte_idx_fp.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1103 lines, 29 scanned public declarations, 1 scanned internal function definitions, and 27 structured function-comment blocks.
- Compute a fingerprint of a term suitable for fingerprint indexing. A fingerprint is a vector of individual samples for positions p, where the result is t|p->f_code if p is a position in t, BELOW_VAR (=LONG_MIN) if p<=q, t|q=Xn, 0 otherwise.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `TermFPSampleHO` strips lambda heads only while descending a non-empty position; the root sample of a lambda returns the lambda symbol itself. Preserve this for compatibility, but consider a normalized diagnostic wrapper once LFHO output behavior is covered.
- The higher-order trailing-argument branch uses `term->arity + TypeGetMaxArity(term->type)`, not the remaining unapplied type arity. This can report a DB-lambda/trailing-argument sample even for positions beyond the apparent applied arguments; preserve the bound unless reference tests justify a compatibility switch.
- `TermFPSampleHO` assumes typed terms when it checks trailing type arity. Rust should keep that precondition explicit instead of silently treating missing types as first-order absence.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
