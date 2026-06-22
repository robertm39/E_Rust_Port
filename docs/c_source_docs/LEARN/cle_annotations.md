<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_annotations

## Source Files

- [LEARN/cle_annotations.h](../../../eprover/LEARN/cle_annotations.h)
- [LEARN/cle_annotations.c](../../../eprover/LEARN/cle_annotations.c)

## Purpose

Functions and datatype for dealing with and administrating annotations. the GNU Lesser General Public License. <1> Fri Jul 16 20:45:49 MET DST 1999

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `Annotation_p`

### Macros And Constants

- `ANNOTATIONS_MERGE_ALL`
- `ANNOTATION_DEFAULT_SIZE`
- `AnnotationCount(anno)`
- `AnnotationLength(anno)`
- `AnnotationValues(anno)`
- `CLE_ANNOTATIONS`

### Globals

- None found in the source scan.

### Exported Functions

- `Annotation_p AnnotationAlloc(void)`
- `Annotation_p AnnotationParse(Scanner_p in, long expected)`
- `DDArrayElement(((anno)->val1.p_val), 0) void AnnotationTreeFree(Annotation_p tree)`
- `double AnnotationEval(Annotation_p anno, double weights[])`
- `long AnnotationListParse(Scanner_p in, Annotation_p *tree, long expected)`
- `long AnnotationMerge(Annotation_p *tree, Annotation_p collect, PStack_p sources)`
- `void AnnotationCombine(Annotation_p res, Annotation_p new_anno)`
- `void AnnotationFree(Annotation_p junk)`
- `void AnnotationListPrint(FILE* out, Annotation_p tree)`
- `void AnnotationPrint(FILE* out, Annotation_p anno)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `AnnotationAlloc`: Allocate a NumTree-Cell with accompanying ClauseStats-Cell.
- `AnnotationFree`: Free an annotation.
- `AnnotationTreeFree`: Free an annotation tree.
- `AnnotationParse`: Parse a single annotation of the proof:(number[,number]*) and return a pointer to it.
- `AnnotationListParse`: Parse the list of annotations into the tree. Return number of items parsed.
- `AnnotationPrint`: Print a single annotation.
- `AnnotationListPrint`: Print the list of annotations.
- `AnnotationCombine`: Combine two annotations into one, i.e. compute the weighted average of each value. Results are returned in res.
- `AnnotationMerge`: Given an annotation tree and a stack of annotation keys (proof numbers), add the annotation vector to *collect. Return number of annotations found.
- `AnnotationEval`: Return an evaluation for the annotation. The annotation is sum anno[i+1]*weights[i] (due to the special meaning of anno[0]. Yes, this sucks rocks, but I'm to lazy to fix this now!)

### Dependencies

- `"cle_annotations.h"`
- `<cio_basicparser.h>`
- `<clb_numtrees.h>`
- `<clb_pdarrays.h>`

### Compile-Time Conditions

- `CLE_ANNOTATIONS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_annotations.h`, `LEARN/cle_annotations.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 493 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 10 structured function-comment blocks.
- Functions and datatype for dealing with and administrating annotations. the GNU Lesser General Public License. <1> Fri Jul 16 20:45:49 MET DST 1999
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
