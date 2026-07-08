<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_stratpar

## Source Files

- [PROVER/e_stratpar.c](../../../eprover/PROVER/e_stratpar.c)

## Purpose

Hack for the SLB category of CASC-2017 - run 8 E's in parallel on a given problem the GNU Lesser General Public License.

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
- `<ccl_relevance.h>`
- `<ccl_sine.h>`
- `<cco_batch_spec.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<clb_defines.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_stratpar.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 213 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Hack for the SLB category of CASC-2017 - run 8 E's in parallel on a given problem the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/e_stratpar.rs` and `src/bin/e_stratpar.rs` port the standalone wrapper over the Rust process-control owner, including C's eight hard-coded `AutoSched` children, ignored optional prover argument, first proof-output replay, no-proof child messages, final `% SZS status GaveUp` when every child exits without a recognized proof status, and the C `OutClose(GlobalOut)` final flush/error check on the execution path.

### Change Later

- The optional `<path-to-eprover>` positional argument is advertised and accepted, but the C implementation leaves `prover` fixed to `"eprover"` and never reads the second positional argument. Preserve this until drop-in tests are stable, then either honor the argument or remove it from the cleaned interface.
- The usage error reports `e_ltb_runner` instead of `e_stratpar`. Keep the typo visible for compatibility audits, but treat it as a candidate for a future user-facing cleanup.
- The executable is intentionally hard-coded to eight `AutoSched` children and halves the global hard time limit for each child. That matches the CASC-2017 SLB hack, but later process scheduling should share configuration and orchestration with the normal auto-schedule path.
- `process_options()` exits directly for help/version before `main()` reaches `OutClose(GlobalOut)`, while ordinary execution closes stdout through `OutClose`. Keep that split in compatibility wrappers; a cleaned API should make flush/close ownership explicit.
<!-- END MANUAL REVIEW: c_source_docs -->
