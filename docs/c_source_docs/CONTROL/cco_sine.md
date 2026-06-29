<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_sine

## Source Files

- [CONTROL/cco_sine.h](../../../eprover/CONTROL/cco_sine.h)
- [CONTROL/cco_sine.c](../../../eprover/CONTROL/cco_sine.c)

## Purpose

Data types and definitions for supporting SinE-like specification filtering. <1> Thu May 10 08:35:26 CEST 2012 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `StructFOFSpecCell`
- `StructFOFSpec_p`

### Macros And Constants

- `CCO_SINE`
- `StructFOFSpecCellAlloc()`
- `StructFOFSpecCellFree(junk)`
- `StructFOFSpecResetShared(ctrl)`

### Globals

- None found in the source scan.

### Exported Functions

- `StructFOFSpec_p StructFOFSpecAlloc(void)`
- `StructFOFSpec_p StructFOFSpecCreate(TB_p terms)`
- `long ProofStateSinE(ProofState_p state, char* filter)`
- `long StructFOFSpecCollectFCode(StructFOFSpec_p ctrl, FunCode f_code, PStack_p res_formulas)`
- `long StructFOFSpecGetProblem(StructFOFSpec_p ctrl, AxFilter_p filter, PStack_p res_clauses, PStack_p res_formulas)`
- `long StructFOFSpecParseAxioms(StructFOFSpec_p ctrl, PStack_p axfiles, IOFormat parse_format, char* default_dir)`
- `void StructFOFSpecAddProblem(StructFOFSpec_p ctrl, ClauseSet_p clauses, FormulaSet_p formulas, bool trim)`
- `void StructFOFSpecBacktrackToSpec(StructFOFSpec_p ctrl)`
- `void StructFOFSpecDestroy(StructFOFSpec_p ctrl)`
- `void StructFOFSpecFree(StructFOFSpec_p ctrl)`
- `void StructFOFSpecInitDistrib(StructFOFSpec_p ctrl, bool trim)`

## Implementation Notes

### Internal Functions

- `find_auto_sine`
- `sine_get_filter`

### Source-Level Behavior

- `sine_get_filter`: Given a filter string (a definition or a name), return the described filter. Initialize filters with a set of filters including the described one.
- `find_auto_sine`: Given a proof state, return the name of the "best" SInE-Strategy, or NULL if SInE is not recommended.
- `StructFOFSpecCreate`: Create a FOF spec, given the term bank (and thus the sig).
- `StructFOFSpecAlloc`: Allocate a Structures problem data structure.
- `StructFOFSpecDestroy`: Dissassemble and Free the FOFSpec, but leave term bank and signature alone.
- `StructFOFSpecFree`: Free a StructFOFSpec data structure.
- `StructFOFSpecCollectFCode`: Push all formulas that contain f_code onto result. Return number of formulas found. Ignores clauses (clauses are deprecated here).
- `StructFOFSpecParseAxioms`: Initialize a StructFOFSpeclCell by parsing all the include files in axfiles.
- `StructFOFSpecInitDistrib`: Initialize the f_distrib element of an otherwise initialized structured problem cell.
- `ProofStateSinE`: Apply SinE with the specified filter to the proofstate (in particular state->f_axioms and state->axioms). This is destructive. Returns number of axioms deleted.

### Dependencies

- `"cco_sine.h"`
- `<ccl_formulafunc.h>`
- `<ccl_proofstate.h>`
- `<ccl_sine.h>`
- `<che_rawspecfeatures.h>`

### Compile-Time Conditions

- `CCO_SINE`
- `NEVER_DEFINED`

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

Source files reviewed: `CONTROL/cco_sine.h`, `CONTROL/cco_sine.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 781 lines, 13 scanned public declarations, 2 scanned internal function definitions, and 10 structured function-comment blocks.
- SInE axiom-selection control layer; relevance thresholds and symbol-frequency flow must match clause-level SInE support.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- In the normal compiled branch, `sine_get_filter` always starts from `AxFilterDefaultSet`. It parses a direct filter definition only when the second token is `(`, otherwise it looks up the entire input string as a default filter name and reports a usage error with the default-name list on failure. This means `Threshold(10)` is accepted as an anonymous extra filter, while `custom=Threshold(10)` is rejected despite `AxFilterDefParse` being able to parse that shape.
- `find_auto_sine` uses generated raw problem-class limits, the mask `-aaaaaaa`, and parallel `raw_class`/`raw_sine` arrays embedded directly in `cco_sine.c`; it returns no filter for problems with no conjectures or hypotheses even if the generated class table would otherwise match.
- `ProofStateSinE(state, "Auto")` calls that lookup, prints `% No SInE strategy applied` through `GlobalOut` when the lookup returns `NULL`, otherwise prints the selected filter name and destructively prunes both formula and clause owners through the selected ax-filter.
- `src/heuristics/axfilter.rs` ports the normal-build `sine_get_filter` resolution behavior for default names, direct unnamed definitions, and unknown-name diagnostics. `src/prover/eprover.rs` now applies the C control-layer ordering for threshold and clause-side GSinE on represented clause axioms: resolve the filter, print `% SinE strategy is ...`, run SInE before relevance pruning and initial docs, and fold deleted clauses into the combined relevancy/SInE statistics count. LambdaDef, formula-owner pruning, and pointer-preserving mixed selected-axiom movement remain pending until stable clause/formula owners are available.
- Change-later candidate: the generated Auto SInE class table and hard-coded limits are source-embedded C data. Once compatibility and update tests cover this path, consider deriving the Rust data from generated schedule metadata or a checked build-time extractor instead of preserving another hand-copied table.
- Change-later candidate: the dead `NEVER_DEFINED` `sine_get_filter` branch would parse named definitions differently and would not preload the default set for direct definitions. Rust preserves the live branch; if a cleaned API allows inline named SInE definitions later, make that an explicit extension instead of silently changing C's normal-build option semantics.
- Change-later candidate: `ProofStateSinE` implements selection by deregistering the old proof-state sets, pushing borrowed selected pointers into stacks, allocating fresh clause/formula sets, and moving selected objects into the new owners. Rust's threshold filter avoids that handle problem because selection is all-or-nothing for current clause owners, and executable clause-side GSinE currently moves by selected clause identifier; formula-aware GSinE/LambdaDef should use stable handles instead of cloning selected axioms when ported.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
