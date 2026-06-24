<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_clausefeatures

## Source Files

- [HEURISTICS/che_clausefeatures.h](../../../eprover/HEURISTICS/che_clausefeatures.h)
- [HEURISTICS/che_clausefeatures.c](../../../eprover/HEURISTICS/che_clausefeatures.c)

## Purpose

Functions for determining various features of clauses. the GNU Lesser General Public License. <1> Mon Sep 28 19:17:50 MET DST 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_CLAUSEFEATURES`
- `ClauseAddVarDistribution(clause, dist_array)`

### Globals

- None found in the source scan.

### Exported Functions

- `EqnListAddVarDistribution((clause)->literals, (dist_array)) long ClauseCountVariableSet(Clause_p clause)`
- `FunCode EqnAddVarDistribution(Eqn_p eqn, PDArray_p dist_array)`
- `FunCode EqnListAddVarDistribution(Eqn_p list, PDArray_p dist_array)`
- `FunCode TermAddVarDistribution(Term_p term, PDArray_p dist_array)`
- `int ClauseCountExtSymbols(Clause_p clause, Sig_p sig, long min_arity)`
- `long ClauseCountMaximalLiterals(Clause_p clause)`
- `long ClauseCountMaximalTerms(Clause_p clause)`
- `long ClauseCountSingletonSet(Clause_p clause)`
- `long ClauseCountUnorientableLiterals(Clause_p clause)`
- `long ClauseTPTPDepthInfoAdd(Clause_p clause, long* depthmax, long* depthsum, long* count)`
- `void ClauseInfoPrint(FILE* out, Clause_p clause)`
- `void ClauseLinePrint(FILE* out, Clause_p clause, bool printinfo)`
- `void ClausePropInfoPrint(FILE* out, Clause_p clause)`

## Implementation Notes

### Internal Functions

- `eqn_tptp_depth_info_add`
- `term_depth_info_add`

### Source-Level Behavior

- `term_depth_info_add`: Change term depth to depthsum, adapt depthmax, increase count by one. Return the new max.
- `eqn_tptp_depth_info_add`: Add term depth info according to TPTP interpretation (all literals are conventional, equations are interpreted as equal(t1, t2)).
- `ClauseCountExtSymbols`: Return the number of different external function symbols in clause.
- `TermAddVarDistribution`: Count the variable occurences in term. Return the largest (negated) variable f_count.
- `EqnAddVarDistribution`: As TermAddVarDistribution(), but for equations.
- `EqnListAddVarDistribution`: As TernAddVarDistribution, for lists of equations.
- `ClauseCountVariableSet`: Return the number of different variables in clause.
- `ClauseCountSingletonSet`: Return the number of different singleton variables in clause.
- `ClauseCountMaximalTerms`: Given an clause, return the number of maximal terms in maximal literals.
- `ClauseCountMaximalLiterals`: Given an clause, return the number of maximal literals.
- `ClauseCountUnorientableLiterals`: Given an clause, return the number of unorientable literals.
- `ClauseTPTPDepthInfoAdd`: Add the term depth information according to TPTP interpretation (see eqn_tptp_depth_info_add()).
- `ClauseInfoPrint`: Print a lot of information about clause in the form info(d0,...,dn) with d0: Clause ident (number) d1: Proof depth d2: Proof length d3: Symbol count d4: Clause depth d5: Literal number d6: Number of variable occurences d7: Number of different variables
- `ClauseLinePrint`: Print the clause and potential information on a single line.
- `ClausePropInfoPrint`: Print a clause and certain statistical information about it as a comment.

### Dependencies

- `"che_clausefeatures.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `CHE_CLAUSEFEATURES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_clausefeatures.h`, `HEURISTICS/che_clausefeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 620 lines, 13 scanned public declarations, 2 scanned internal function definitions, and 15 structured function-comment blocks.
- Functions for determining various features of clauses. the GNU Lesser General Public License. <1> Mon Sep 28 19:17:50 MET DST 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### C Behaviors To Revisit After Compatibility

- `ClauseInfoPrint` labels field `d6` as variable occurrences, but computes it through `ClauseWeight(..., max_term_multiplier=0, vweight=1, fweight=1, ...)`, so the value includes the corrected equality-predicate contribution and follows orientation/maximality weight semantics rather than a direct variable-occurrence count.
- `ClauseLinePrint` adds exactly ` COMCHARRAW ` plus `ClauseInfoPrint` when `printinfo` is true, then always writes a trailing newline. The Rust helper ports that assembly over caller-rendered clause text; exact full-wrapper parity still needs the `ClausePrint` family.
- `ClausePropInfoPrint` emits its statistics through fixed `%6ld`/`%6d` fields and the compile-time `COMCHAR` prefix. The Rust stats-block helper keeps the spacing and accepts an explicit comment prefix; wire the final wrapper to the eventual output-format configuration before exposing it as full `ClausePropInfoPrint`.
<!-- END MANUAL REVIEW: c_source_docs -->
