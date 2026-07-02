<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / epclanalyse

## Source Files

- [PROVER/epclanalyse.c](../../../eprover/PROVER/epclanalyse.c)

## Purpose

Read a PCL protocol and collect and print a number of statistics on the protocol. the GNU Lesser General Public License. <1> Thu Feb 28 13:45:43 MET 2002

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

- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<cio_tempfile.h>`
- `<e_version.h>`
- `<pcl_propanalysis.h>`
- `<stdio.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
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

Source files reviewed: `PROVER/epclanalyse.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 265 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a PCL protocol and collect and print a number of statistics on the protocol. the GNU Lesser General Public License. <1> Thu Feb 28 13:45:43 MET 2002
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/epclanalyse.rs` and `src/bin/epclanalyse.rs` port the standalone `epclanalyse` executable over the existing Rust PCL2 full-protocol owner and property-analysis functions. The port covers `-h`/`--help`, long-only `--version`, `-v`/`--verbose`, `-o`/`--output-file`, `-s`/`--silent`, default stdin input through `-`, TPTP-format PCL parsing, strict end-of-input checking, property-statistic aggregation through `PCLProtPropAnalyse`-style logic, C-shaped summary/representative-step output, two-line `SysError`-style file-open diagnostics, and C `OutClose` wording on final flush failure.

### Change Later

- Unlike nearby tools such as `epclextract`, C `epclanalyse` defines `--version` without a `-V` shorthand. Rust preserves that option table; add `-V` only behind an explicit compatibility decision.
- `--silent` sets C's global `OutputLevel` to `0`, but the property-summary output is not level-gated. Rust accepts it as a no-op for command-line compatibility; a later cleanup can remove or document it if no shared output layer consumes the side effect.
- The C help text uses a legacy 2002-2009 support-tool footer and obsolete URL instead of the shared modern `E_FOOTER`. Rust preserves that visible text for this executable; consider moving old support-tool footers behind a shared compatibility helper later.
- `epclanalyse` inherits `PCLProtPropDataPrint`'s zero-denominator average formulas and unconditional representative/metric printing. Empty protocols and formula-only protocols can hit assertions, null pointers, or invalid clause-union reads in C after the summary header; Rust keeps the visible arithmetic shape but makes representative rendering total. After drop-in compatibility is secured, the C path should report unavailable representatives explicitly.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
