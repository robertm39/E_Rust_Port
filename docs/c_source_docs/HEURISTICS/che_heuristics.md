<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_heuristics

## Source Files

- [HEURISTICS/che_heuristics.h](../../../eprover/HEURISTICS/che_heuristics.h)
- [HEURISTICS/che_heuristics.c](../../../eprover/HEURISTICS/che_heuristics.c)

## Purpose

High-Level interface functions to the heuristics module. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_HEURISTICS`

### Globals

- None found in the source scan.

### Exported Functions

- `HCB_p GetHeuristic(char* source, HCBARGUMENTS)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `GetHeuristic`: Given a string (either a name or a Heuristic-Definition), return a corresponding HCB.

### Dependencies

- `"che_heuristics.h"`
- `"che_new_autoschedule.h"`
- `<che_proofcontrol.h>`

### Compile-Time Conditions

- `CHE_HEURISTICS`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_heuristics.h`, `HEURISTICS/che_heuristics.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 192 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Heuristic-control block parsing and selection; command-line strategy syntax depends on this behavior.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
