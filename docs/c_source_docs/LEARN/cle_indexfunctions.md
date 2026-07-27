<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_indexfunctions

## Source Files

- [LEARN/cle_indexfunctions.h](../../../eprover/LEARN/cle_indexfunctions.h)
- [LEARN/cle_indexfunctions.c](../../../eprover/LEARN/cle_indexfunctions.c)

## Purpose

Functions and data types realizing simple index functions. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `IndexTermCell`
- `IndexTerm_p`
- `IndexType`
- `TSMIndexCell`
- `TSMIndex_p`
- `tree`

### Macros And Constants

- `CLE_INDEXFUNCTIONS`
- `IndexDynamicDepth`
- `IndexTermCellAlloc()`
- `IndexTermCellFree(junk)`
- `TSMIndexCellAlloc()`
- `TSMIndexCellFree(junk)`

### Globals

- `extern char* IndexFunNames[]`

### Exported Functions

- `IndexTerm_p IndexTermAlloc(Term_p term, PatternSubst_p subst, long key)`
- `TSMIndex_p TSMIndexAlloc(IndexType type, int depth, TB_p bank, PatternSubst_p subst)`
- `int GetIndexType(char* name)`
- `int IndexTermCompareFun(const void* term1, const void* term2)`
- `long TSMIndexFind(TSMIndex_p index, Term_p term, PatternSubst_p subst)`
- `long TSMIndexInsert(TSMIndex_p index, Term_p term)`
- `void IndexTermFree(IndexTerm_p junk, TB_p bank)`
- `void TSMIndexFree(TSMIndex_p junk)`
- `void TSMIndexPrint(FILE* out, TSMIndex_p index, int depth)`

## Implementation Notes

### Internal Functions

- `any_term_top`

### Source-Level Behavior

- `any_term_top`: Return a term top as specified by the parameters.
- `GetIndexType`: Given a string, return the proper index type or -1.
- `GetIndexName`: Return the name of an IndexType.
- `IndexTermAlloc`: Return an initialized index term.
- `IndexTermFree`: Free a IndexTerm.
- `IndexTermCompareFun`: Compare two index terms (as patterns).
- `TSMIndexAlloc`: Return an initialized index cell.
- `TSMIndexFree`: Free a TSMIndex.
- `TSMIndexFind`: Return an index for term (-1 if no index exists);
- `TSMIndexInsert`: Insert the term/index association into the index. This duplicates some stuff from TSMIndexFind() to save on term copies. The patterns substitution is taken to be consistent for all terms inserted or to be inserted into index and is not passed along. Returns index assigned.
- `TSMIndexPrint`: Print a TSM-Index (as a comment) in the form "keyno:keyobj"*

### Dependencies

- `"cle_indexfunctions.h"`
- `<clb_objtrees.h>`
- `<clb_simple_stuff.h>`
- `<cle_patterns.h>`
- `<cle_termtops.h>`

### Compile-Time Conditions

- `CLE_INDEXFUNCTIONS`

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

Source files reviewed: `LEARN/cle_indexfunctions.h`, `LEARN/cle_indexfunctions.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 779 lines, 16 scanned public declarations, 1 scanned internal function definitions, and 11 structured function-comment blocks.
- Functions and data types realizing simple index functions. the GNU Lesser General Public License.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Compatibility Notes

- `index_counter` is file-static and only used for debug identities in `TSMIndexPrint`; it is incremented on allocation and is not reset by `TSMIndexFree` or TSM admin cleanup.
- `TSMIndexCell` and `IndexTermCell` store shared `PatternSubst_p` pointers. `IndexTermCompareFun` asserts total substitutions, and the object-tree ordering assumes those substitutions remain stable after insertion.
- Rust mirrors those shared pointers with `Rc<PatternSubst>` in `TSMIndex` and `IndexTerm`; regression tests pin pointer identity. This avoids recursively deep-copying the substitution and its signature while retaining owned public constructors for callers that need an independent substitution.
- `TSMIndexFind` obtains an `IndexSymbol` key through the side-effect-free `PatSymbValue` accessor. Rust symbol-index lookup likewise borrows the stored substitution and does not clone it; profiling a 1,000-term recursive corpus showed that the former per-lookup deep copy accounted for 51.59% of executed instructions.
- `IndexTermAlloc` only stores the term pointer and substitution pointer. Despite comments about references and bank mutation, `IndexTermFree` just asserts the bank pointer and frees the wrapper cell.
- `IndexEmpty` can be allocated and found against, returning `-1`, but insertion asserts; higher-level parsers reject it for active TSM weight parameters.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
