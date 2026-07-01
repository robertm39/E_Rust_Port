<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_steps

## Source Files

- [PCL2/pcl_steps.h](../../../eprover/PCL2/pcl_steps.h)
- [PCL2/pcl_steps.c](../../../eprover/PCL2/pcl_steps.c)

## Purpose

PCL steps. the GNU Lesser General Public License. <1> Thu Mar 30 17:52:53 MET DST 2000 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLStepCell`
- `PCLStepProperties`
- `PCLStep_p`
- `logic`

### Macros And Constants

- `PCLNoWeight`
- `PCLStepCellAlloc()`
- `PCLStepCellFree(junk)`
- `PCLStepDelProp(clause, prop)`
- `PCLStepGiveProps(clause, prop)`
- `PCLStepIsAnyPropSet(clause, prop)`
- `PCLStepIsClausal(step)`
- `PCLStepIsFOF(step)`
- `PCLStepIsShell(step)`
- `PCLStepPrint(out, step)`
- `PCLStepQueryProp(clause, prop)`
- `PCLStepSetProp(clause, prop)`
- `PCL_PROOF_DIST_DEFAULT`
- `PCL_PROOF_DIST_INFINITY`
- `PCL_PROOF_DIST_UNKNOWN`
- `PCL_STEPS`

### Globals

- `extern bool SupportShellPCL`

### Exported Functions

- `PCLStepProperties PCLParseExternalType(Scanner_p in)`
- `PCLStep_p PCLStepParse(Scanner_p in, TB_p bank)`
- `char * PCLPropToTSTPType(PCLStepProperties props)`
- `int PCLStepIdCompare(const void* s1, const void* s2)`
- `void PCLPrintExternalType(FILE* out, PCLStepProperties props)`
- `void PCLStepFree(PCLStep_p junk)`
- `void PCLStepPrintExample(FILE* out, PCLStep_p step, long id, long proof_steps, long total_steps)`
- `void PCLStepPrintExtra(FILE* out, PCLStep_p step, bool data)`
- `void PCLStepPrintFormat(FILE* out, PCLStep_p step, bool data, OutputFormatType format)`
- `void PCLStepPrintLOP(FILE* out, PCLStep_p step)`
- `void PCLStepPrintTPTP(FILE* out, PCLStep_p step)`
- `void PCLStepPrintTSTP(FILE* out, PCLStep_p step)`
- `void PCLStepResetTreeData(PCLStep_p step, bool just_weights)`

## Implementation Notes

### Internal Functions

- `print_shell_pcl_warning`

### Source-Level Behavior

- `print_shell_pcl_warning`: Print a warning that a shell PCL step was encountered where a normal one was expected.
- `PCLStepFree`: Free a PCL step.
- `PCLParseExternalType`: Parse a list of type annotations for PCL steps and return a property word that can be used with SetProp() to set all necessary properties (the type field and the lemma bit).
- `PCLStepParse`: Parse a PCL step.
- `PCLPrintExternalType`: Print the type(s) of a PCL step encoded in props.
- `PCLStepPrintExtra`: Print a PCL step.
- `PCLPropToTSTPType`: Given PCL properties, return the best string describing the type.
- `PCLStepPrintTSTP`: Print a PCL step in TSTP format.
- `PCLStepPrintLOP`: Print the logical part of a PCL step as a LOP clause or formula (where TPTP core syntax has to stand in for missing LOP syntac).
- `PCLStepPrintFormat`: Print a PCL step in the requested format.
- `PCLStepPrintExampe`: Print a PCL step in the correct format for an E example file for pattern-based learning. The format is as follows: id: (pd, su, sf, gu, gs, ss):clause where currently id is meaningless (a survivor from the old output format), pd is the proof distance, su, sf, gu, gs are the relative number of simplified or generated proof/nonproof clauses, and ss is 0 (it u...
- `PCLStepIdCompare`: Compare two PCL steps by idents (forPTreeObj-Operations).
- `PCLStepResetTreeData`: Reset all counters and size data elements in the step to 0.

### Dependencies

- `"pcl_steps.h"`
- `<pcl_expressions.h>`

### Compile-Time Conditions

- `NEVER_DEFINED`
- `PCL_STEPS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_steps.h`, `PCL2/pcl_steps.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 799 lines, 18 scanned public declarations, 1 scanned internal function definitions, and 13 structured function-comment blocks.
- PCL steps. the GNU Lesser General Public License. <1> Thu Mar 30 17:52:53 MET DST 2000 New
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Rust support is in `src/pcl2/steps.rs`, covering the shared PCL step property word, proof-distance constants, external type parse/print helpers, TSTP role mapping, identifier comparison through `PclId`, and the `PCLStepResetTreeData` analysis-counter reset behavior.
- Initial full-step support now covers `PCLStep` representation, full-id parsing, clausal/formula/shell logical content, full-expression justifications, full-step extras accepting `SQString|Name|PosInt`, PCL/TSTP/TPTP/LOP rendering, shell-step warning side-channel rendering through explicit warning-aware wrappers, example-line rendering over analysis counters, and format dispatch for the C-supported output formats.

### Change Later

- `SupportShellPCL` is a process-global switch that changes parser behavior for both full and mini PCL steps. Rust has not introduced the global; later parser integration should make shell support an explicit session/config option unless executable compatibility requires a process-wide flag.
- The header comment beside `PCLType1 = CPType1` says `/* 256 */`, but the current `CPType1` value in `ccl_clauses.h` is `1024`. Rust follows the actual compiled value and records the stale comment as source drift.
- `PCLParseExternalType` accepts `que`, but its fallback `CheckInpId` message lists only `conj|neg|lemma`. Rust preserves that diagnostic surface in the helper; a cleaned API should include every accepted token.
- Empty external type fields parse as `PCLTypeAxiom`, while `PCLPropToTSTPType` maps a plain axiom type to `plain` unless `PCLIsInitial` is also set. This role distinction is easy to lose when refactoring proof-output code.
- `PCLStepResetTreeData(step, false)` resets analysis counters and also clears `PCLIsLemma|PCLIsMarked`; `just_weights=true` resets only the two weight fields. Keep this property side effect visible when lemma-analysis code is ported.
- `PCLStepParse` calls `PCLStepResetTreeData(handle, false)` before assigning `handle->properties`, so the reset helper clears bits in an uninitialized property field before the parser overwrites it. Rust initializes tree data directly and sets parsed properties afterward; a cleaned C-compatible API should separate data initialization from property mutation.
- Shell-step logical-format printers call `Warning(...)` on stderr and also emit a comment-line omission marker to the requested output stream. Rust now exposes the warning side channel through explicit warning-aware wrappers while keeping the pure string renderers side-effect-light; executable integration should route those wrappers through the session warning stream for PCL tool compatibility.
- `PCLStepPrintExample` delegates clause rendering to global `OutputFormat` through `ClausePrint`, even though example files are otherwise a fixed learning-data format. Rust currently uses the LOP-shaped clause rendering available in the local API; revisit once the process-global output-format compatibility shim exists.
- `PCLStepPrintTPTP` prints formula-backed `input_formula(...)` without appending a final period. Rust preserves this string shape; a future TPTP compatibility audit should verify whether consumers expect the missing terminator.
<!-- END MANUAL REVIEW: c_source_docs -->
