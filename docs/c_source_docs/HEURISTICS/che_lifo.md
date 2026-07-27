<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_lifo

## Source Files

- [HEURISTICS/che_lifo.h](../../../eprover/HEURISTICS/che_lifo.h)
- [HEURISTICS/che_lifo.c](../../../eprover/HEURISTICS/che_lifo.c)

## Purpose

LIFO-Evaluation of a clause (unfair!) the GNU Lesser General Public License. <1> Mon Jun 22 15:28:23 MET DST 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_LIFO`

### Globals

- None found in the source scan.

### Exported Functions

- `WFCB_p LIFOEvalInit(ClausePrioFun prio_fun)`
- `WFCB_p LIFOEvalParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double LIFOEvalCompute(void* data, Clause_p clause)`
- `void LIFOEvalExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `LIFOEvalInit`: Return an initialized WFCB for FIFO evaluation.
- `LIFOEvalParse`: Parse a lifo-declaration.
- `LIFOEvalCompute`: Compute an evaluation for a clause.
- `LIFOEvalExit`: Free the data entry in a LIFO WFCB.

### Dependencies

- `"che_lifo.h"`
- `<che_wfcb.h>`

### Compile-Time Conditions

- `CHE_LIFO`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_lifo.h`, `HEURISTICS/che_lifo.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 202 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- LIFO-Evaluation of a clause (unfair!) the GNU Lesser General Public License. <1> Mon Jun 22 15:28:23 MET DST 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `LIFOEvalCompute` ignores the clause pointer and decrements the mutable double counter before returning it, so the first computed value is `-1.0`. Rust mirrors this with `LifoEvaluator` state and WFCB-backed evaluation.
- The C comment for `LIFOEvalInit` says FIFO evaluation even though the function initializes LIFO state; Rust follows the implementation and treats the comment as stale.

### Rust Port Status Notes

- `src/heuristics/lifo.rs` ports LIFO evaluator allocation, stateful compute behavior, WFCB initialization, priority-function parsing inside brackets, and the no-op exit hook over owned Rust state.

### Change Later

- C heap-allocates a single `double` for the LIFO counter and frees it through a callback even though the state is just one scalar. Rust stores the scalar directly inside the typed evaluator; keep that safer ownership shape unless a future C-ABI compatibility layer needs callback-owned opaque data.
- The `LIFOEvalInit` structured comment says FIFO evaluation in the LIFO module. Treat it as a stale comment and avoid copying it into user-facing docs.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
