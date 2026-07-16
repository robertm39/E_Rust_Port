<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / checkproof

## Source Files

- [PROVER/checkproof.c](../../../eprover/PROVER/checkproof.c)

## Purpose

Read a PCL protocol and try to verify it using a selected prover. the GNU Lesser General Public License. <1> Fri Apr 7 16:14:02 MET DST 2000 New

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
- `<pcl_proofcheck.h>`
- `<stdio.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

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

Source files reviewed: `PROVER/checkproof.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 346 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Proof-checking application; preserve accepted proof formats and diagnostic behavior.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/checkproof.rs` and `src/bin/checkproof.rs` port the standalone `checkproof` executable over the existing Rust PCL2 proof-checking core. The port covers exact C-shaped full help text, long-only `--version`, `-v`/`--verbose`, `-o`/`--output-file` including `-o -`, `-s`/`--silent`, `-l`/`--output-level`, `-p`/`--prover-type`, `-x`/`--executable`, `-t`/`--prover-cpu-limit`, default stdin input through `-`, TPTP-format UPCL2 parsing with C-compatible shared external variable-name mapping for compressed clause input, C-compatible shell-PCL rejection, strict end-of-input checks, external E/Otter/SPASS verification dispatch, release-compatible `scheme-setheo` failure classification, full-FOF warning routing, C-shaped temporary/file-open/output-close diagnostics, and the C-shaped final verification summary. The permanent comparison matrix uses paired companion C/Rust `eprover` binaries for real success/failure checks and portable shell adapters for exact output-level-3 traces plus legacy Otter/SPASS problem/marker paths.

### Change Later

- C exposes `--version` without a `-V` shorthand here, unlike some newer E tools. Rust preserves that table; add a short alias only as an explicit non-compatibility-mode cleanup.
- `print_help(FILE* out)` prints its option table to `stdout` instead of the `out` parameter. The executable only calls it with `stdout`, so Rust keeps the user-visible behavior without preserving the misdirected helper API.
- The C executable mutates global `OutputFormat`, `EqnUseInfix`, and `ClausesHaveLocalVariables` while selecting Otter/SPASS checking and parsing UPCL2. Rust keeps the parser effect in explicit `PclStepParseOptions`/`ClauseParseOptions` and keeps output effects localized in proof-check rendering paths; audit again if shared global output-format state becomes part of the public Rust API.
- `checkproof.c` relies on the process-global `SupportShellPCL` default staying false, while other PCL tools such as `epclextract` enable it explicitly. Rust keeps this as an explicit per-executable parser option; a cleaned interface should avoid hidden global defaults for proof-protocol dialect selection.
- `scheme-setheo` is accepted by C but has no switch arm in `PCLStepCheck`. Debug C builds assert for a generated check problem; the normal `NDEBUG` release build removes that assertion and returns the initialized `CheckFail`, while assumptions still pass and split steps remain `CheckNotImplemented`. Rust matches the release result rather than its previous unchecked classification; remove or rename the option only after compatibility mode can report deprecated options.
- C installs SIGTERM/SIGINT handlers mainly to clean temporary prover problem files. Rust uses owned temporary-file registration/removal around each prover run and still sets the equivalent handlers for executable compatibility; a later process-management layer could make cleanup ownership explicit and avoid global signal setup in library-facing paths.
- `main()` calls `OpenGlobalOut(outname)` before inserting the default `-` argument and before scanner creation, so output redirection can create or truncate a file even if proof input later fails. Rust keeps this side effect for compatibility; a future user-facing mode could delay or atomically commit output.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
