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

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
