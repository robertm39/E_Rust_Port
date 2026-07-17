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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for ownership and compatibility equivalence on 2026-07-17.

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

### Rust Port Status

- Initial Rust support is in `src/pcl2/ministeps.rs`, covering a discriminated owned logic representation, numeric mini-step parsing, clausal minification, formula-backed steps, explicit shell-step parsing, PCL/TSTP rendering, and format dispatch for the C-supported PCL/TSTP branches. The enum prevents C's untagged clause/formula union from being read under inconsistent property bits.
- Rust uses explicit `PclMiniStepParseOptions` for `SupportShellPCL` instead of a mutable process-global flag. C initializes the global to false and only `epclextract` assigns true; the corresponding Rust executable call sites pass those same values, and regression coverage proves consecutive disabled/enabled/disabled parses cannot leak state.
- C stores the parsing `TB_p` for formula printing but still receives the bank as an argument for clause printing. Its only production mini-step print callers are the owning mini-protocol, which passes its own bank. Rust makes that invariant explicit: mini-step terms are safe shared handles, the step retains no raw owner pointer, and mini-protocol rendering always supplies its bank for either logic variant.

### Change Later

- `PCLMiniStepParse` rejects compound identifiers after parsing the first integer, even though full PCL steps accept `PCLId` lists. Rust preserves this mini-mode restriction exactly; any syntax expansion remains tracked by `E_Rust_Port-j76.4.955`.
- The C parser's shell-step behavior depends on the global `SupportShellPCL` flag and a second colon immediately after the type field. Rust preserves the grammar and executable modes with a call-scoped option; reconsidering process-global mutation remains tracked by `E_Rust_Port-j76.4.956` and `E_Rust_Port-j76.4.979`.
- `PCLMiniStepFree` asserts that `junk->id` is nonzero even though standalone `PCLMiniStepParse` accepts `0`. Rust retains the parse result but uses ordinary infallible destruction; protocol parsing still begins only on `PosInt`. Reproducing or tightening the contradictory C invariant remains tracked by `E_Rust_Port-j76.4.957` and the protocol-level `E_Rust_Port-j76.4.950`.
- Mini-step extras accept only `SQString`, while full PCL step extras also accept `Name|PosInt`. Rust retains that narrower grammar; any expansion remains tracked by `E_Rust_Port-j76.4.958`.
- `PCLMiniStepPrintTSTP` prints shell clausal steps with an empty formula slot, producing a double-comma shape such as `cnf(id,plain,,just).`. Rust preserves the exact output; any cleaned proof-object format remains tracked by `E_Rust_Port-j76.4.959`.
<!-- END MANUAL REVIEW: c_source_docs -->
