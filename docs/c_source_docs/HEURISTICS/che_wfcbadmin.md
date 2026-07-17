<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_wfcbadmin

## Source Files

- [HEURISTICS/che_wfcbadmin.h](../../../eprover/HEURISTICS/che_wfcbadmin.h)
- [HEURISTICS/che_wfcbadmin.c](../../../eprover/HEURISTICS/che_wfcbadmin.c)

## Purpose

Functions for administrating and parsing sets of weight functions. the GNU Lesser General Public License. <1> Tue Dec 8 22:27:02 MET 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `WFCBAdminCell`
- `WFCBAdmin_p`

### Macros And Constants

- `CHE_WFCB_ADMIN`
- `WFCBAdminCellAlloc()`
- `WFCBAdminCellFree(junk)`

### Globals

- `extern char* WeightFunParseFunNames[]`

### Exported Functions

- `WFCBAdmin_p WFCBAdminAlloc(void)`
- `WFCB_p WFCBAdminFindWFCB(WFCBAdmin_p set, char* name)`
- `WFCB_p WeightFunParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WeightFunParseFun GetWeightFunParseFun(char* name)`
- `char* WeightFunDefParse(WFCBAdmin_p set, Scanner_p in, OCB_p ocb, ProofState_p state)`
- `long WFCBAdminAddWFCB(WFCBAdmin_p set, char* name, WFCB_p wfcb)`
- `long WeightFunDefListParse(WFCBAdmin_p set, Scanner_p in, OCB_p ocb, ProofState_p state)`
- `void WFCBAdminFree(WFCBAdmin_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `WFCBAdminAlloc`: Allocate an empty initialized WFCBAdminCell.
- `WFCBAdminFree`: Free a WFCBAdminCell. Will also free stored wfcb's and names.
- `WFCBAdminAddWFCB`: Add a WFCB under a given name to the WFCB-Set. Return index.
- `WFCBAdminFindWFCB`: Given a name and a WFCB-Set, return the matching WFCB (or NULL). Always returns the last WFCB with the same name, so you can redefine predefined weight functions!
- `GetWeightFunParseFun`: Given a name of a weight function, return a parse function for it.
- `WeightFunParse`: Parse a weight function.
- `WeightFunDefParse`: Parse a weight function definition and add it to the set. Returns a pointer to the name.
- `WeightFunDefListParse`: Parse a list of weight function definitions. Return number of entries parsed.

### Dependencies

- `"che_wfcbadmin.h"`
- `<che_clauseweight.h>`
- `<che_dagweight.h>`
- `<che_diversityweight.h>`
- `<che_fifo.h>`
- `<che_funweights.h>`
- `<che_gdweight.h>`
- `<che_learning.h>`
- `<che_levweight.h>`
- `<che_lifo.h>`
- `<che_orientweight.h>`
- `<che_prefixweight.h>`
- `<che_random.h>`
- `<che_simweight.h>`
- `<che_strucweight.h>`
- `<che_termweight.h>`
- `<che_tfidfweight.h>`
- `<che_treeweight.h>`
- `<che_varweights.h>`

### Compile-Time Conditions

- `CHE_WFCB_ADMIN`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_wfcbadmin.h`, `HEURISTICS/che_wfcbadmin.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 503 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for administrating and parsing sets of weight functions. the GNU Lesser General Public License. <1> Tue Dec 8 22:27:02 MET 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Compatibility Notes

- `WeightFunParse` dispatches by parallel compile-time tables of names and parser function pointers. Rust keeps the table order and dispatches every current parser entry. Production proof control now supplies clause axioms, represented formula axioms, and the live signature through an explicit `WeightParseContext`; this covers every option-defined and inline WFCB, including relevance-level formula consumers and TSM/TSMR signature consumers. Context-free wrappers remain low-level APIs and diagnose state-dependent parser names instead of fabricating an owner. The production boundary and 47/47 executable C/Rust matrix are recorded in [`experiments/2026-07-17-056-weight-parser-context-matrix/FINDINGS.md`](../../../experiments/2026-07-17-056-weight-parser-context-matrix/FINDINGS.md).
- `WeightFunDefParse` duplicates explicit definition names before `WFCBAdminAddWFCB`, which duplicates them again, and it passes stack-local anonymous names only because `WFCBAdminAddWFCB` immediately duplicates the string. Rust stores owned `String` names directly; revisit only if strategy parsing allocation cost becomes visible.
- C parser failures are fatal diagnostics from the current scanner position. Rust returns `Diagnostic` values but keeps the token-consumption boundary explicit so strategy parsing can later choose whether to abort like C.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
