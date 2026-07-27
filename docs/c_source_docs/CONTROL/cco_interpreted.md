<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_interpreted

## Source Files

- [CONTROL/cco_interpreted.h](../../../eprover/CONTROL/cco_interpreted.h)
- [CONTROL/cco_interpreted.c](../../../eprover/CONTROL/cco_interpreted.c)

## Purpose

Code for handling (some) interpreted symbols. Initially, this will only deal with answer predicates (some of which may be false in otherwise empty clauses). Once things have shaken out, I expect more general solutions here...

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_INTERPRETED`
- `XXXCellAlloc()`
- `XXXCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `int ClauseEvaluateAnswerLits(Clause_p clause)`
- `void ClausePrintAnswer(FILE* out, Clause_p clause, ProofState_p state)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `answer_lit_print`: Print the answer in an answer literal nicely. At the moment, we print proper answer literals as tuples of answers, all others as-is.
- `ClausePrintAnswer`: If the clause has only answer literals, print the answer.
- `ClauseEvaluateAnswerLits`: "Evaluate" the answer literals, i.e. remove all single-answer lits if the clause is otherwise empty. Return number of removed literals.

### Dependencies

- `"cco_interpreted.h"`
- `<ccl_proofstate.h>`

### Compile-Time Conditions

- `CCO_INTERPRETED`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_interpreted.h`, `CONTROL/cco_interpreted.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 262 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Code for handling (some) interpreted symbols. Initially, this will only deal with answer predicates (some of which may be false in otherwise empty clauses). Once things have shaken out, I expect more general solutions here...
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ClausePrintAnswer` emits `Theorem` once, prints `# SZS answers Tuple [...]` for semantically false non-empty answer clauses, and mutates `status_reported` so the final proof banner does not print another SZS status. Rust now records and drains that C-shaped answer output for the supported clause-list proof-search path; full proof-object extraction roots and derivation printing remain deferred.
- `ClauseEvaluateAnswerLits` only removes simple answer literals when the whole clause is semantically false, then recomputes positive/negative literal counts and records answer-evaluation proof metadata. Rust exposes the local clause mutation/count recomputation and uses it from the staged `ProcessClause` answer-return path.

### Change Later

- `ClauseEvaluateAnswerLits` asserts the clause is not demodulation/subsumption indexed and, when the clause still belongs to a set, decrements the owning set's literal count. Rust currently calls the helper on owned/extracted clauses only, so no containing set accounting is needed; add indexed/set-owned integration if later call sites evaluate answer literals in place.
- `ClausePrintAnswer` reaches back into proof-state status reporting while also formatting an interpreted-symbol result. Rust preserves the one-shot status side effect for compatibility, but the C layering would be cleaner if answer formatting returned structured proof-search events instead of mutating global/final-output state directly.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
