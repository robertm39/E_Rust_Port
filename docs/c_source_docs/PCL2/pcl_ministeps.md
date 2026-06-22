<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_ministeps

## Source Files

- [PCL2/pcl_ministeps.h](../../../eprover/PCL2/pcl_ministeps.h)
- [PCL2/pcl_ministeps.c](../../../eprover/PCL2/pcl_ministeps.c)

## Purpose

Maximally compact PCL steps, only for special purpose applications. the GNU Lesser General Public License. <1> Wed Jul 10 20:44:47 MEST 2002 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLMiniStepCell`
- `PCLMiniStep_p`
- `logic`

### Macros And Constants

- `PCLMiniStepCellAlloc()`
- `PCLMiniStepCellFree(junk)`
- `PCL_MINISTEPS`

### Globals

- None found in the source scan.

### Exported Functions

- `PCLMiniStep_p PCLMiniStepParse(Scanner_p in, TB_p bank)`
- `void PCLMiniStepFree(PCLMiniStep_p junk)`
- `void PCLMiniStepPrint(FILE* out, PCLMiniStep_p step, TB_p bank)`
- `void PCLMiniStepPrintFormat(FILE* out, PCLMiniStep_p step, TB_p bank, OutputFormatType format)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCLMiniStepFree`: Free a PCLMini step.
- `PCLMiniStepParse`: Parse a PCLMini step.
- `PCLMiniStepPrint`: Print a PCLMini step.
- `PCLMiniStepPrintTSTP`: Print a PCLMini step in TSTP format.
- `PCLMiniStepPrintFormat`: Print a PCL step in the requested format.

### Dependencies

- `"pcl_ministeps.h"`
- `<pcl_expressions.h>`
- `<pcl_miniclauses.h>`
- `<pcl_steps.h>`

### Compile-Time Conditions

- `PCL_MINISTEPS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_ministeps.h`, `PCL2/pcl_ministeps.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 354 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Maximally compact PCL steps, only for special purpose applications. the GNU Lesser General Public License. <1> Wed Jul 10 20:44:47 MEST 2002 New
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
