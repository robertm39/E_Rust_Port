<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_axiomscan

## Source Files

- [HEURISTICS/che_axiomscan.h](../../../eprover/HEURISTICS/che_axiomscan.h)
- [HEURISTICS/che_axiomscan.c](../../../eprover/HEURISTICS/che_axiomscan.c)

## Purpose

Declarations for functions recognizing certain axioms (e.g. AC axioms). the GNU Lesser General Public License. <1> New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_AXIOMSCAN_H`
- `FAIL_ON(x)`

### Globals

- None found in the source scan.

### Exported Functions

- `FunCode DetectAssociativity(Clause_p clause)`
- `FunCode DetectCommutativity(Clause_p clause)`
- `bool ClauseScanAC(Sig_p sig, Clause_p clause)`
- `bool ClauseSetScanAC(Sig_p sig, ClauseSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DetectCommutativity`: If clause is a comutativity axiom for some function symbol, return this symbol. Otherwise return 0.
- `DetectAssociativity`: If clause is a associativity for some function symbol, return this symbol. Otherwise return 0.
- `ClauseScanAC`: Enter AC properties induced by clause into sig. Return true if at least a C-axiom has beed detected.
- `ClauseSetScanAC`: Enter AC properties induced by clause set into sig. Return true if at least a C-axiom has beed detected.

### Dependencies

- `"che_axiomscan.h"`
- `<ccl_clausesets.h>`
- `<cle_indexfunctions.h>`

### Compile-Time Conditions

- `CHE_AXIOMSCAN_H`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_axiomscan.h`, `HEURISTICS/che_axiomscan.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 309 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Declarations for functions recognizing certain axioms (e.g. AC axioms). the GNU Lesser General Public License. <1> New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
