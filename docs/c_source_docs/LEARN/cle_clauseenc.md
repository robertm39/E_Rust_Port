<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_clauseenc

## Source Files

- [LEARN/cle_clauseenc.h](../../../eprover/LEARN/cle_clauseenc.h)
- [LEARN/cle_clauseenc.c](../../../eprover/LEARN/cle_clauseenc.c)

## Purpose

Functions for dealing with term representations of clauses. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_CLAUSEENC`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p FlatEncodeClauseListRep(TB_p bank, PStack_p list)`
- `Term_p FlatRecodeRecClauseRep(TB_p bank,Term_p clauserep)`
- `Term_p ParseClauseTermRep(Scanner_p in, TB_p bank, bool flat)`
- `Term_p RecEncodeClauseListRep(TB_p bank, PStack_p list)`
- `Term_p TermEncodeEqnList(TB_p bank, Eqn_p list, bool flat)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FlatEncodeClauseListRep`: Take a PStack wich describes a clause in a given order and compute the flat term representation of it.
- `RecEncodeClauseListRep`: Take a PStack wich describes a clause in a given order and compute the recursive term representation of it.
- `TermEncodeEqnList`: Encode an eqnlist (as might be parsed with EqnListParse()) as a term.
- `FlatRecodeRecClauseRep`: Take a recursive clause encoding and generate a corresponding flat one. This is a simple, not a particularly efficient implementation.
- `ParseClauseTermRep`: Parse a clause representation in literal list format (other formats are unsuitable because literal order matters!) and return a term representation of it.

### Dependencies

- `"cle_clauseenc.h"`
- `<cle_patterns.h>`

### Compile-Time Conditions

- `CLE_CLAUSEENC`

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

Source files reviewed: `LEARN/cle_clauseenc.h`, `LEARN/cle_clauseenc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 318 lines, 5 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Functions for dealing with term representations of clauses. the GNU Lesser General Public License.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
