<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_fifo

## Source Files

- [HEURISTICS/che_fifo.h](../../../eprover/HEURISTICS/che_fifo.h)
- [HEURISTICS/che_fifo.c](../../../eprover/HEURISTICS/che_fifo.c)

## Purpose

FIFO-Evaluation of a clause/ the GNU Lesser General Public License. <1> Sat Jul 5 02:28:25 MET DST 1997 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_FIFO`

### Globals

- None found in the source scan.

### Exported Functions

- `WFCB_p FIFOEvalInit(ClausePrioFun prio_fun)`
- `WFCB_p FIFOEvalParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double FIFOEvalCompute(void* data, Clause_p clause)`
- `void FIFOEvalExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FIFOEvalInit`: Return an initialized WFCB for FIFO evaluation.
- `FIFOEvalParse`: Parse a fifo-declaration.
- `FIFOEvalCompute`: Compute an evaluation for a clause.
- `FIFOEvalExit`: Free the data entry in a FIFO WFCB.

### Dependencies

- `"che_fifo.h"`
- `<che_wfcb.h>`

### Compile-Time Conditions

- `CHE_FIFO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_fifo.h`, `HEURISTICS/che_fifo.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 202 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- FIFO-Evaluation of a clause/ the GNU Lesser General Public License. <1> Sat Jul 5 02:28:25 MET DST 1997 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
