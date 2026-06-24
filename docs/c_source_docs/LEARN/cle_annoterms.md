<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_annoterms

## Source Files

- [LEARN/cle_annoterms.h](../../../eprover/LEARN/cle_annoterms.h)
- [LEARN/cle_annoterms.c](../../../eprover/LEARN/cle_annoterms.c)

## Purpose

Terms and term sets with annotation lists. the GNU Lesser General Public License. Create: Tue Jul 20 17:22:38 MET DST 1999

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AnnoSetCell`
- `AnnoSet_p`
- `AnnoTermCell`
- `AnnoTerm_p`

### Macros And Constants

- `AnnoSetCellAlloc()`
- `AnnoSetCellFree(junk)`
- `AnnoTermCellAlloc()`
- `AnnoTermCellFree(junk)`
- `AnnoTermFreeNoRef(junk)`
- `CLE_ANNOTERMS`

### Globals

- None found in the source scan.

### Exported Functions

- `AnnoSet_p AnnoSetAlloc(TB_p bank)`
- `AnnoSet_p AnnoSetParse(Scanner_p in, TB_p bank, long expected)`
- `AnnoTerm_p AnnoTermAlloc(Term_p term, Annotation_p annos)`
- `AnnoTerm_p AnnoTermAllocNoRef(Term_p term, Annotation_p annos)`
- `AnnoTerm_p AnnoTermParse(Scanner_p in, TB_p bank, long expected)`
- `bool AnnoSetAddTerm(AnnoSet_p set, AnnoTerm_p term)`
- `bool AnnoSetComputePatternSubst(PatternSubst_p subst, AnnoSet_p set)`
- `long AnnoSetFlatten(AnnoSet_p set, PStack_p set_idents)`
- `long AnnoSetRecToFlatEnc(TB_p bank, AnnoSet_p set)`
- `long AnnoSetRemoveByIdent(AnnoSet_p set, long set_ident)`
- `long AnnoSetRemoveExceptIdentList(AnnoSet_p set, PStack_p set_idents)`
- `void AnnoSetFree(AnnoSet_p junk)`
- `void AnnoSetFreeNoRef(AnnoSet_p junk)`
- `void AnnoSetNormalizeFlatAnnos(AnnoSet_p set)`
- `void AnnoSetPrint(FILE* out, AnnoSet_p set)`
- `void AnnoTermFree(TB_p bank, AnnoTerm_p junk)`
- `void AnnoTermPrint(FILE* out, TB_p bank, AnnoTerm_p term, bool fullterms)`
- `void AnnoTermRecToFlatEnc(TB_p bank, AnnoTerm_p term)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `annotation_collect_max`: Set max_values[i] := max(max_values[i], anno(i+1)
- `annotation_normalize`: Divide the values in anno by the corresponding max_value.
- `AnnoTermAlloc`: Allocate an initialized AnnoTermCell
- `AnnoTermFree`: Free an annotated term. Does not free the substitution. Carefull...it _will_ free the annotation.
- `AnnoTermParse`: Parse an annotated term and return it.
- `AnnoTermPrint`: Print an annotated term.
- `AnnoTermRecToFlatEnc`: Take an annotated term encoding a clause in recursive format and recode it into flat format.
- `AnnoSetAlloc`: Allocate an empty, initialized set of annotated terms.
- `AnnoSetFree`: Free a set of annotated terms.
- `AnnoSetAddTerm`: Insert term into set, where term is expected to belong to the sets term bank. Returns true if the term is new, false otherwise.
- `AnnoSetParse`: Parse a set of annotated terms.
- `AnnoSetPrint`: Print a set of annotated terms.
- `AnnoSetComputePatternSubst`: Compute a pattern subst for all terms in set. Return true if subst has been modified.
- `AnnoSetRemoveByIdent`: Given a set of terms and an example source id, remove all annotations from the source, and any terms that have no remaining annotations. Return number of terms deleted.
- `AnnoSetRemoveExceptIdentList`: Given a set of terms and a stack of idents, remove all annotations from the source whose are not in the stack. Remove terms without annotations as well. If set_idents is ANNOTATIONS_MERGE_ALL, do nothing. Returns number of terms deleted.
- `AnnoSetFlatten`: Given a set of annotated terms and a stack of idents (or ANNOTATIONS_MERGE_ALL), compute a single flat annotation for all terms and remove terms which have no annotation. Return number of terms remaining.
- `AnnoSetNormalizeFlatAnnos`: Normalize the annotations, i.e. divide each annotation value by the maximum value of this annotation for all terms.
- `AnnoSetRecToFlatEnc`: Recode all terms in set from recursive to flat format. Returns number of terms.

### Dependencies

- `"cle_annoterms.h"`
- `<cle_annotations.h>`
- `<cle_clauseenc.h>`

### Compile-Time Conditions

- `CLE_ANNOTERMS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_annoterms.h`, `LEARN/cle_annoterms.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 785 lines, 22 scanned public declarations, 0 scanned internal function definitions, and 18 structured function-comment blocks.
- Terms and term sets with annotation lists. the GNU Lesser General Public License. Create: Tue Jul 20 17:22:38 MET DST 1999
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `AnnoSetAlloc` eagerly creates the equality and recursive-clause representation symbols in the supplied term-bank signature (`$eq`, `$neq`, `$or`, `$cnil`) even when the set remains empty. Preserve this side effect when parsing knowledge-base annotation sets.
- `AnnoTermParse` delegates term syntax to `TBTermParse`, then consumes `:`, an annotation list with an exact expected element count, and `.`. `AnnoSetParse` keeps parsing while the current token is `TermStartToken`, so an extra term-like token after the last annotated term is parsed as a malformed annotated term rather than ignored as trailing data.
- `AnnoTermPrint` writes `term : annotations.` and `AnnotationListPrint` concatenates annotation entries without separators; set printing prefixes entries with a blank line and `# Annotated terms:`.
- `AnnoSetComputePatternSubst` traverses every stored annotated term and calls `PatternTermCompute` even if earlier terms already changed the substitution; the return value is the OR of all per-term change results.
- `AnnoSetFlatten` documents a return value of "number of terms remaining", but the local `count` is never incremented and the function always returns zero. Rust preserves this result for the ported flatten helper.
- `AnnoSetRemoveExceptIdentList` checks `PStackGetSP(stack)` where `stack` is the NumTree traversal stack, not the caller's `set_idents` stack. Rust exposes the useful id-retention helper with an explicit id-list bound because the current sorted-map owner has no equivalent raw traversal stack; revisit only if learned-data reference tests expose dependence on the C accident.
- `AnnoSetRecToFlatEnc` mutates each stored annotated term in place and returns the number of terms visited.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
