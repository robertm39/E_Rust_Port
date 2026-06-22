<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_termtops

## Source Files

- [LEARN/cle_termtops.h](../../../eprover/LEARN/cle_termtops.h)
- [LEARN/cle_termtops.c](../../../eprover/LEARN/cle_termtops.c)

## Purpose

Compute the various term tops for given (shared!) terms. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_TERMTOPS`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p AltTermTop(Term_p term, int depth, VarBank_p freshvars)`
- `Term_p CSTermTop(Term_p term, int depth, VarBank_p freshvars)`
- `Term_p ESTermTop(Term_p term, int depth, VarBank_p freshvars)`
- `Term_p TermTop(Term_p term, int depth, VarBank_p freshvars)`

## Implementation Notes

### Internal Functions

- `alt_rek_term_top`
- `rek_term_top`
- `term_del_prop_level`
- `term_set_prop_at_level`

### Source-Level Behavior

- `term_del_prop_level`: Clear prop in all terms reachable by a path of length <= depth from the root node.
- `term_set_prop_at_level`: Set prop in all terms reachable by a path of length == depth from the root node.
- `rek_term_top`: Return the term top of term at level i.
- `alt_rek_term_top`: Return the alternate term top of term at level i.
- `term_top_marked`: Copy the term top up to the nodes marked with TPOpFlag
- `TermTop`: Compute top(term, depth).
- `AltTermTop`: Compute top'(term, depth). See above. Requires that bindings are NULL in term.
- `CSTermTop`: Return the compact shared top term of t at level depth.
- `ESTermTop`: Return the extended shared top term of t at level depth.

### Dependencies

- `"cle_termtops.h"`
- `<cle_patterns.h>`
- `<cte_termbanks.h>`

### Compile-Time Conditions

- `CLE_TERMTOPS`

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

Source files reviewed: `LEARN/cle_termtops.h`, `LEARN/cle_termtops.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 462 lines, 4 scanned public declarations, 4 scanned internal function definitions, and 9 structured function-comment blocks.
- Compute the various term tops for given (shared!) terms. the GNU Lesser General Public License.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
