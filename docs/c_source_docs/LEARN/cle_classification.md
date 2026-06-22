<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_classification

## Source Files

- [LEARN/cle_classification.h](../../../eprover/LEARN/cle_classification.h)
- [LEARN/cle_classification.c](../../../eprover/LEARN/cle_classification.c)

## Purpose

Functions for using TSM's as classification tools on terms. the GNU Lesser General Public License. <1> Fri Aug 13 20:26:50 MET DST 1999 New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_CLASSIFICATION`

### Globals

- None found in the source scan.

### Exported Functions

- `bool TSMClassifiedTermCheck(TSMAdmin_p admin, FlatAnnoTerm_p term)`
- `double TSMTermClassify(TSMAdmin_p admin, Term_p term, PatternSubst_p subst)`
- `long TSMClassifySet(TSMAdmin_p admin, FlatAnnoSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TSMTermClassify`: Classify a term with a TSM, i.e. return -1 if the evaluation is lower than limit, +1 otherwise
- `TSMClassifiedTermCheck`: Classify term on the tsm and compare it with the original classification. Return true if they match, false otherwise.
- `TSMClassifySet`: Classify all terms in set with the TSM, return number of successes.

### Dependencies

- `"cle_classification.h"`
- `<cle_tsm.h>`

### Compile-Time Conditions

- `CLE_CLASSIFICATION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_classification.h`, `LEARN/cle_classification.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 200 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Functions for using TSM's as classification tools on terms. the GNU Lesser General Public License. <1> Fri Aug 13 20:26:50 MET DST 1999 New
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
