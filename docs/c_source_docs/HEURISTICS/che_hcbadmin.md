<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_hcbadmin

## Source Files

- [HEURISTICS/che_hcbadmin.h](../../../eprover/HEURISTICS/che_hcbadmin.h)
- [HEURISTICS/che_hcbadmin.c](../../../eprover/HEURISTICS/che_hcbadmin.c)

## Purpose

Functions for administrating and parsing sets of heuristics. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `HCBAdminCell`
- `HCBAdmin_p`

### Macros And Constants

- `CHE_HCB_ADMIN`
- `HCBAdminCellAlloc()`
- `HCBAdminCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `HCBAdmin_p HCBAdminAlloc(void)`
- `HCB_p HCBAdminFindHCB(HCBAdmin_p set, char* name)`
- `HCB_p HeuristicParse(Scanner_p in, WFCBAdmin_p wfcbs, OCB_p ocb, ProofState_p state)`
- `long HCBAdminAddHCB(HCBAdmin_p set, char* name, HCB_p hcb)`
- `long HeuristicDefListParse(HCBAdmin_p set, Scanner_p in, WFCBAdmin_p wfcbs, OCB_p ocb, ProofState_p state)`
- `long HeuristicDefParse(HCBAdmin_p set, Scanner_p in, WFCBAdmin_p wfcbs, OCB_p ocb, ProofState_p state)`
- `void HCBAdminFree(HCBAdmin_p junk)`

## Implementation Notes

### Internal Functions

- `parse_single_wfcb_item`

### Source-Level Behavior

- `parse_single_wfcb_item`: Parse a single wfcb-item "name(steps)" and insert it into the HCB.
- `HCBAdminAlloc`: Allocate an empty initialized HCBAdminCell.
- `HCBAdminFree`: Free a HCBAdminCell. Will also free stored hcb's and names.
- `HCBAdminAddHCB`: Add a HCB under a given name to the HCB-Set. Return index.
- `HCBAdminFindHCB`: Given a name and a HCB-Set, return the matching HCB (or NULL). Always returns the last HCB with the same name, so you can redefine predefined heuristics!
- `HeuristicParse`: Parse a heuristic.
- `HeuristicDefParse`: Parse a heuristics definition and add it to the set.
- `HeuristicDefListParse`: Parse a list of heuristics_definitions definitions.

### Dependencies

- `"che_hcbadmin.h"`
- `<che_hcb.h>`

### Compile-Time Conditions

- `CHE_HCB_ADMIN`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_hcbadmin.h`, `HEURISTICS/che_hcbadmin.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 394 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for administrating and parsing sets of heuristics. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `HCBAdminCell` keeps names and HCB pointers in parallel `PStack`s, and `HCBAdminFindHCB` scans backward. Duplicate names intentionally shadow earlier definitions; keep this redefinition behavior even if the Rust registry later gains map-style lookup.
- `HeuristicParse` parses a non-empty parenthesized list of positive-step items. Each item accepts either `*` or `.` between the step count and WFCB specifier, so both `2*Weight` and `2.Weight` are valid strategy syntax.
- `parse_single_wfcb_item` treats an identifier followed by `(` as an inline anonymous weight-function definition through `WeightFunDefParse`; an identifier followed by `=` becomes a named WFCB definition; a bare identifier is looked up in the WFCB admin. Parser cleanup should preserve these three cases.
- `HeuristicDefParse` uses the name `Default` when the definition starts directly with `(`. `HeuristicDefListParse` returns the existing stack size if it parses nothing, but the zero-based index of the last parsed definition otherwise; Rust preserves that mixed return value.
- `HCBAdminFree` owns and frees stored HCBs and duplicated names, but each HCB still only stores admin-owned WFCB references. Rust mirrors this with owned `HcbCell`s containing WFCB admin handles.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
