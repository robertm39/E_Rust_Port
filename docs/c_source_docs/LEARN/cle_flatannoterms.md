<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_flatannoterms

## Source Files

- [LEARN/cle_flatannoterms.h](../../../eprover/LEARN/cle_flatannoterms.h)
- [LEARN/cle_flatannoterms.c](../../../eprover/LEARN/cle_flatannoterms.c)

## Purpose

Terms with only an evaluation and a counter left. the GNU Lesser General Public License. <1> Mon Aug 9 12:32:53 MET DST 1999 New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FlatAnnoSetCell`
- `FlatAnnoSet_p`
- `FlatAnnoTermCell`
- `FlatAnnoTerm_p`

### Macros And Constants

- `CLE_FLATANNOTERMS`
- `FlatAnnoSetCellAlloc()`
- `FlatAnnoSetCellFree(junk)`
- `FlatAnnoTermCellAlloc()`
- `FlatAnnoTermCellFree(junk)`
- `FlatAnnoTermFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(FlatAnnoSetCell*)SizeMalloc(sizeof(FlatAnnoSetCell)) SizeFree(junk, sizeof(FlatAnnoSetCell)) void FlatAnnoTermPrint(FILE* out, FlatAnnoTerm_p term, Sig_p sig)`
- `(FlatAnnoTermCell*)SizeMalloc(sizeof(FlatAnnoTermCell)) SizeFree(junk, sizeof(FlatAnnoTermCell)) FlatAnnoTerm_p FlatAnnoTermAlloc(Term_p term, double eval, double eval_weight, long sources)`
- `FlatAnnoSet_p FlatAnnoSetAlloc(void)`
- `bool FlatAnnoSetAddTerm(FlatAnnoSet_p set, FlatAnnoTerm_p term)`
- `double FlatAnnoSetEvalAverage(FlatAnnoSet_p set)`
- `double FlatAnnoSetEvalWeightedAverage(FlatAnnoSet_p set)`
- `long FlatAnnoSetFlatten(FlatAnnoSet_p set, FlatAnnoSet_p to_flatten)`
- `long FlatAnnoSetSize(FlatAnnoSet_p fset)`
- `long FlatAnnoSetTranslate(FlatAnnoSet_p flatset, AnnoSet_p set, double weights[])`
- `long FlatAnnoTermFlatten(FlatAnnoSet_p set, FlatAnnoTerm_p term)`
- `void FlatAnnoSetFree(FlatAnnoSet_p junk)`
- `void FlatAnnoSetPrint(FILE* out, FlatAnnoSet_p set, Sig_p sig)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FlatAnnoTermAlloc`: Return a flatly annotated term.
- `FlatAnnoTermPrint`: Print a flatly annotated term "t:eval." (mostly for debugging, I suppose)
- `FlatAnnoSetAlloc`: Allocate a flatly annotated term set.
- `FlatAnnoSetFree`: Free a set of flatly annotated terms.
- `FlatAnnoSetPrint`: Print a set o flatly annotated terms (mostly for debugging, I suppose)
- `FlatAnnoSetAddTerm`: Add a flatly annotated term to a set. If the term already exists, merge the annotations and free the original annotateted term.
- `FlatAnnoSetTranslate`: Given a set of annotated terms with exactly one annotation per term, generate a corrsponding flatly annotated term set.
- `FlatAnnoSetSize`: Return the number of terms in a flatanno-set (counting sources).
- `FlatAnnoTermFlatten`: Generate a fresh annoterm for each subterm of term (inheriting the original annotation with modiefied weight) and insert it into set. Returns number of terms created.
- `FlatAnnoSetFlatten`: For all terms in to_flatten and all subterms, insert them into set. Return number of terms created.
- `FlatAnnoSetEvalAverage`: Return the average of all evaluation for terms in set.
- `FlatAnnoSetEvalWeightedAverage`: Return the weighted average of all evaluation for terms in set.

### Dependencies

- `"cle_flatannoterms.h"`
- `<clb_ddarrays.h>`
- `<cle_annoterms.h>`

### Compile-Time Conditions

- `CLE_FLATANNOTERMS`

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

Source files reviewed: `LEARN/cle_flatannoterms.h`, `LEARN/cle_flatannoterms.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 559 lines, 16 scanned public declarations, 0 scanned internal function definitions, and 12 structured function-comment blocks.
- Terms with only an evaluation and a counter left. the GNU Lesser General Public License. <1> Mon Aug 9 12:32:53 MET DST 1999 New
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `FlatAnnoSetTranslate` assumes each annotated term has been flattened to exactly one annotation, uses `AnnotationEval(weights)` as the stored evaluation, keeps annotation slot `0` as `eval_weight`, and casts that same count to `long` for `sources`.
- `FlatAnnoSetSize` returns the sum of `sources`, not the number of unique flat term nodes. Classification success counts use the same source-weighted convention.

### Change Later

- Audit whether source counts should remain a double-to-long cast after compatibility is secured. Fractional counts, negative counts, and values outside `long` range are all implementation-defined or surprising in C-shaped behavior.
- Split the overloaded flat annotation fields into named concepts (`class_eval`, `eval_weight`, source count) once learned-data compatibility tests make it safe to move away from the compact C struct shape.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
