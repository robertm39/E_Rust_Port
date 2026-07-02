<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / epclextract

## Source Files

- [PROVER/epclextract.c](../../../eprover/PROVER/epclextract.c)

## Purpose

Read a PCL protocol and print all steps that are needed to print "proof", "final", or "extract" steps. the GNU Lesser General Public License.

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
- `<pcl_miniprotocol.h>`
- `<pcl_protocol.h>`
- `<stdio.h>`

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

Source files reviewed: `PROVER/epclextract.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 383 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a PCL protocol and print all steps that are needed to print "proof", "final", or "extract" steps. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/epclextract.rs` and `src/bin/epclextract.rs` port the standalone `epclextract` executable over the existing Rust PCL2 full and mini protocol owners. The port covers `-h`/`--help`, `-V`/`--version`, `-v`/`--verbose`, `-f`/`--fast-extract`, `-C`/`--forward-comments`, `-c`/`--competition-framing`, `-n`/`--no-extract`, `--tstp-out`, `--tptp3-out`, `-o`/`--output-file`, and `-s`/`--silent`; default stdin input through `-`; TPTP-format PCL parsing; end-of-input checking; full-protocol recursive proof-step marking; mini-protocol fast suffix marking; optional comment forwarding before proof-clause output, including multi-file order; shell and formula-valued no-extract rendering; PCL/TSTP proof-clause printing; C-shaped SZS framing text, including the missing period after the closing `CNFRefutation` line; two-line `SysError`-style file-open diagnostics; and C `OutClose` wording on final flush failure.

### Change Later

- The C `FAST_EXIT` compile-time branch skips protocol freeing, `CLStateFree`, output close, and `ExitIO` by calling `exit(0)` immediately after printing. Rust always flushes and unwinds owned state; only add a compatibility knob if a build profile proves that early-exit performance or leak behavior matters.
- `--silent` sets C's global `OutputLevel` to `0`, but this executable's main extraction output is not level-gated. Rust accepts the option as a no-op for command-line compatibility; a later cleaned interface can remove or document it if no shared output layer needs the side effect.
- `--fast-extract` relies on the C mini-protocol assumption that all `proof`/`final`/`extract` seeds are a contiguous suffix of the positive numeric id range. Rust preserves that suffix behavior in the mini-protocol path; do not broaden it to a full scan without reference tests because it changes which proof steps are emitted.
- `--forward-comments` emits comments during parsing, before proof marking and proof-clause printing. That means comments from earlier input files can already be visible if a later file or extraction phase fails. Rust preserves the ordering; future cleanup can isolate streaming comment forwarding from successful extraction output only outside drop-in compatibility mode.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
