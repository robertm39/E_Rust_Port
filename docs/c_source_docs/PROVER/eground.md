<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / eground

## Source Files

- [PROVER/eground.c](../../../eprover/PROVER/eground.c)

## Purpose

Read a problem specification and test wether the problem has a finite Herbrand universe. If yes, create at least all ground instances of clauses necessary for a ground refutation. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- `NAME`

### Globals

- None found in the source scan.

### Exported Functions

- `CLState_p process_options(int argc, char* argv[])`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_grounding.h>`
- `<ccl_splitting.h>`
- `<che_clausesetfeatures.h>`
- `<che_hcb.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `FAST_EXIT`
- `STACK_SIZE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/eground.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 868 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a problem specification and test wether the problem has a finite Herbrand universe. If yes, create at least all ground instances of clauses necessary for a ground refutation. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `--miniscope-limit` is parsed into `miniscope_limit`, its prose advertises a built-in default of 1000, and its no-argument value comes from `TFORM_MINISCOPE_LIMIT_STR` (`2147483648`), but `main()` passes the hard-coded value `1048576` to `FormulaSetCNF2`. Rust preserves all three conflicting surfaces; decide later whether a cleaned non-drop-in mode should make the option control the actual miniscope limit.
- `--local-constraints` sets `constraints`, `local_constraints`, `ClausesHaveDisjointVariables=true`, and `ClausesHaveLocalVariables=false`, but `local_constraints` is not read later in this file. Rust now preserves the parser-visible variable policy with explicit `ClauseParseOptions`; revisit the dead boolean and global parser mutation once grounding compatibility is locked down.
- `FormulaAndClauseSetParse()` owns TSTP wrapper problem-type setup before eground clausifies formulas: `thf(...)` records select higher-order parsing at the wrapper boundary and mixed FO/HO records are rejected through the global problem type. Rust preserves this by leaving the eground run unset until the shared parser sees records; a cleaned grounding API should pass dialect state explicitly.
- `GroundSetPrint()` reaches TSTP clause rendering through `ClausePrint()` and the process-global `OutputFormat`/`problemType`, so THF inputs affect both progress and final ground-instance wrappers without an explicit parameter at the call site. Rust now threads the parsed problem type through the eground adapter; a cleaned C/Rust grounding API should make output format and dialect ordinary arguments.
- `OpenGlobalOut(outname)` runs before the default `-` input is inserted and before any scanner is created, so output paths can be created or truncated even if later input opening or parsing fails. Rust preserves this order; a cleanup mode could stage output before replacing the destination.
- `app_encode` is initialized but unused. Remove it only after the executable option surface and any historical scripts depending on it are audited.
- DIMACS output goes through `GroundSetPrintDimacs`, which delegates non-empty non-unit clause literal printing to `ClausePrintDimacs`; that helper writes literal integers to `stdout` while writing only terminators to the passed `FILE* out`. This is surprising for `--output-file` and should be cleaned only outside drop-in compatibility mode.
- Equational clauses are recoded into predicate literals after a warning, shifting equality semantics onto explicit equality axioms supplied by the user. Keep the warning/output order for compatibility, but consider a clearer user-facing mode after parity.
- `--give-up` estimate-limit handling exits the whole process from inside the grounding helper after printing a failure line to `GlobalOut`, bypassing normal result, statistics, and resource-footer output. Rust preserves the executable behavior while keeping the library-level estimate outcome explicit; a cleaned C API should return this status to the caller instead of calling `exit()`.
- The completion-status switch handles complete, low-memory, and timeout, then asserts on any unknown state. Release builds may not surface a helpful diagnostic for impossible states; a modernized path should use an explicit error or internal invariant check.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/eground.rs` and `src/bin/eground.rs` port the standalone executable wrapper over the shared Rust clause/formula parser bridge and grounding helpers.
- The Rust wrapper parses supported normal input owners into a represented `FormulaSet` without eagerly locking TSTP input to first-order, accepts and validates `--miniscope-limit` while intentionally discarding it like C, runs `FormulaSetPreprocConjectures` plus `FormulaSetCNF2` with C's hard-coded `1048576` miniscope limit and parsed definitional-CNF limit, then continues through the grounding pipeline while carrying the parsed problem type into TSTP progress and final ground-set output.
- The wrapper preserves default stdin through `-`, `OutOpen`-style `-o -` stdout routing, early output-file creation before later input-open failures, two-line `SysError`-style scanner/output open diagnostics, C `OutClose` wording on final flush failure, `--give-up` success-status failure exits, and the C DIMACS split between the configured output stream and raw stdout.
<!-- END MANUAL REVIEW: c_source_docs -->
