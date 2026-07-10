<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_condensation

## Source Files

- [CLAUSES/ccl_condensation.h](../../../eprover/CLAUSES/ccl_condensation.h)
- [CLAUSES/ccl_condensation.c](../../../eprover/CLAUSES/ccl_condensation.c)

## Purpose

Implementation of the condensation rule: C == if C' is a factor of C, C' subsumes C C'

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CondenseFun`

### Macros And Constants

- `CCL_CONDENSATION`

### Globals

- `extern long CondensationAttempts`
- `extern long CondensationSuccesses`

### Exported Functions

- `bool Condense(Clause_p clause)`
- `bool CondenseOnce(Clause_p clause)`

## Implementation Notes

### Internal Functions

- `try_condensation`

### Source-Level Behavior

- `try_condensation`: Try to condense literals l1 and l2 in clause. If successful, modify clause and return true, otherwise return false.
- `CondenseOnce`: Try to condense clause. If successful, simplify the clause, and return true. If not, the clause is unchanged and false is returned.
- `Condense`: Condense a clause as much as possible. Return true if the clause was changed, false otherwise.

### Dependencies

- `"ccl_condensation.h"`
- `"ccl_subsumption.h"`

### Compile-Time Conditions

- `CCL_CONDENSATION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for higher-order complete matching on 2026-07-10.

Source files reviewed: `CLAUSES/ccl_condensation.h`, `CLAUSES/ccl_condensation.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 248 lines, 5 scanned public declarations, 1 scanned internal function definitions, and 3 structured function-comment blocks.
- Implementation of the condensation rule: C == if C' is a factor of C, C' subsumes C C'
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/clauses/condensation.rs` ports `CondenseOnce`, `Condense`, the process-wide attempt/success counters now read by executable statistics, and the candidate-replacement flow through one-way literal unification, duplicate/resolved cleanup, subsumption-order sorting, and mutable-bank candidate subsumption checking for higher-order complete-match parity.
- The Rust port preserves the C gate that only attempts full condensation when there are at least two positive literals or at least two negative literals, while still counting every `Condense` call as an attempt.
- The `DCCondense` derivation-stack side effect is ported when at least one condensation step changes the clause. An opt-in documenting helper emits the represented `DocClauseModificationDefault(..., inf_condense, NULL)` step before pushing `DCCondense`, matching C side-effect order for proof-control callers with a `ProofDocSession`.

### Change Later

- `try_condensation` accepts a `swap` argument, and `CondenseOnce` retries with `swap=true` when either literal is unoriented, but the C helper never reads the argument or swaps literal sides. Rust preserves that no-op retry for compatibility; remove or repair it only after C/Rust comparison tests show the observable behavior intended.
- C replaces `clause->literals` with `cand->literals` and nulls the candidate list before freeing the candidate. Rust uses owned literal transfer, but stable clause-handle/index ownership should still audit this mutation point because live C callers observe the same clause object with a new literal list.
- Condensation statistics are writable process-global `long` variables in C. Rust uses atomic counters for safe test concurrency, but a later statistics subsystem should decide whether these counters remain global, become proof-state-local, or are reset per run.
- C couples condensation proof documentation to the successful fixed-point condensation loop and global output/id state. Rust keeps this behavior behind an explicit output/session wrapper; route future executable call sites through a proof-control-owned session rather than adding hidden globals.
- `DCCondense` has no explicit parent in C; Rust records only the operation entry. Keep this no-parent shape unless proof-object reconstruction proves a synthetic self-parent is required.
<!-- END MANUAL REVIEW: c_source_docs -->
