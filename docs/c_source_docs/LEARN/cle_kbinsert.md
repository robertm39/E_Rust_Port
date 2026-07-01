<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_kbinsert

## Source Files

- [LEARN/cle_kbinsert.h](../../../eprover/LEARN/cle_kbinsert.h)
- [LEARN/cle_kbinsert.c](../../../eprover/LEARN/cle_kbinsert.c)

## Purpose

Functions for implementing the kb-insert operation. the GNU Lesser General Public License. <1> Tue Jul 27 22:10:34 GMT 1999 New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_KBINSERT`

### Globals

- None found in the source scan.

### Exported Functions

- `AnnoTerm_p ParseExampleClause(Scanner_p in, TB_p parse_terms, TB_p internal_terms, long ident)`
- `long KBAxiomsInsert(ExampleSet_p set, ClauseSet_p axioms, Sig_p sig, char* name)`
- `void KBParseExampleFile(Scanner_p in, char* name, ExampleSet_p set, AnnoSet_p examples, Sig_p res_sig)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ParseExampleClause`: Parse an example clause into an annotated term format. Return clause as AnnoTerm or NULL if pattern-computation is to expensive.
- `KBAxiomsInsert`: Insert the example "name" into set and return the ident assigned.
- `KBParseExampleFile`: Parse an example file into the existing structures.

### Dependencies

- `"cle_kbinsert.h"`
- `<cle_annoterms.h>`
- `<cle_kbdesc.h>`

### Compile-Time Conditions

- `CLE_KBINSERT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
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

Source files reviewed: `LEARN/cle_kbinsert.h`, `LEARN/cle_kbinsert.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 283 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Knowledge-base insertion logic; file layout and example metadata are compatibility constraints.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- `KBAxiomsInsert` is ported in `src/learn/kbinsert.rs` as `kb_axioms_insert`, including `set->count + 1` id assignment, clause-set feature-vector extraction, and ignoring the `ExampleSetInsert` result.
- `ParseExampleClause` is ported as `parse_example_clause` for the current simple clause parser, annotation-vector construction, representative pattern-clause computation, recursive clause encoding, signature translation, and insertion into the destination term bank.
- `KBParseExampleFile` remains pending until the Rust annotation set and proof-state learning paths have the same shared term-bank/signature ownership model as the C `AnnoSet_p examples->terms` path.

### Change Later

- `ParseExampleClause` stores the first annotation input twice: slot `1` becomes a proof-count flag (`1` only when the input distance is zero) and slot `2` stores the original distance. Preserve this learned-data layout for compatibility, but consider a named metadata structure after old KB files are covered by regression tests.
- `ParseExampleClause` records `anno->val2.i_val` as one past the last assigned slot, so the implicit annotation length includes the count slot and both special first-value slots. Rust preserves this length; a later cleaned API should distinguish physical vector length from semantic feature count.
- C `PatternClauseCompute` can return false and silently skip the parsed example clause when representative search is too expensive. Rust keeps an `Option` boundary around `parse_example_clause`, but the current `pattern_clause_compute` helper always returns a result; restore the skip once the pattern layer exposes the C cutoff.
- `KBAxiomsInsert` ignores duplicate-id/name insertion failure. If a duplicate name is inserted, the numeric id entry can remain while `set->count` is not advanced. Rust preserves this side effect through `ExampleSet::insert`; a future KB builder should reject duplicate problem names before constructing inconsistent indexes.
- `KBParseExampleFile` frees the temporary axiom term bank and signature immediately after computing numeric features, then parses example clauses through a new term bank sharing the result signature and the annotation set's internal term bank. Rust should keep feature extraction independent from temporary parser ownership, but the full parser needs explicit shared signature/session ownership before it can be a drop-in equivalent.
<!-- END MANUAL REVIEW: c_source_docs -->
