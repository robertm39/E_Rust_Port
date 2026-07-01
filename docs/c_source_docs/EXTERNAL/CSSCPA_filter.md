<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# EXTERNAL / CSSCPA_filter

## Source Files

- [EXTERNAL/CSSCPA_filter.c](../../../eprover/EXTERNAL/CSSCPA_filter.c)

## Purpose

Do CSSCPA stuff (read clauses, accept them into the state if they are necessary or improve it, reject them otherwise). the GNU Lesser General Public License. <1> Mon Apr 10 15:28:48 MET DST 2000

Within the source tree, this unit belongs to `EXTERNAL`. Optional external integration helpers, including CSSCPA filtering support.

Authors noted in source headers: Stephan Schulz, Geoff Sutcliffe

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

- `<cex_csscpa.h>`
- `<cio_commandline.h>`
- `<e_version.h>`
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

Source files reviewed: `EXTERNAL/CSSCPA_filter.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `EXTERNAL` covering 1 source file(s), about 274 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Do CSSCPA stuff (read clauses, accept them into the state if they are necessary or improve it, reject them otherwise). the GNU Lesser General Public License. <1> Mon Apr 10 15:28:48 MET DST 2000
- External integration code. Treat formats, command-line behavior, and temporary files as compatibility surfaces.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- The core CSSCPA state/process-clause behavior and `CSSCPALoop` command parser from `cex_csscpa` are represented in `src/external/csscpa.rs`.
- `src/external/csscpa_filter.rs` and the `CSSCPA_filter` Cargo binary now port the standalone wrapper: C-shaped option parsing for help/version/verbose/output/silent/output-level/rant, stdout or output-file routing, default `-` stdin handling, file scanner setup, sequential input processing over one CSSCPA state, final TSTP positive-unit/negative-unit/non-unit clause-set printing, C `OutClose` output-stream error wording on flush failure, and `InitIO`/`ExitIO` initialization.

### Change Later

- The exact `Please process clauses now, I beg you, great shining CSSCPA, wonder of the world, most beautiful program ever written.` input sequence is an input-buffering workaround. The Rust parser should accept it for compatibility, but a later interface can replace it with an explicit flush/control command.
- `--rant-about-input-buffering` intentionally writes informal complaint text to `stderr`. Keep it isolated in the CLI compatibility layer rather than exposing it through the CSSCPA state API.
- `process_options` mutates process-global `outname`, `OutputLevel`, `Verbose`, `OutputFormat`, and the dummy `app_encode = false` global. Rust should keep those as layered configuration after compatibility tests establish the exact option order and diagnostic wording.
- `main` sets the scanner format to `TSTPFormat`, while the checked-in sample `.csscpa` file uses old `input_clause(...)` statements. The Rust loop accepts that legacy clause form under filter TSTP mode, but reference runs should confirm whether C's scanner/parser accepted it through TSTP mode or through a looser `ClauseParse` dispatch.
- Rust exposes output routing through explicit writers and file creation rather than the process-global `GlobalOut`. This is cleaner for tests, but exact `OpenGlobalOut`/`OutClose` ownership and error wording should still be audited when byte-compatible CLI diagnostics are required.
- Rust file scanners currently load each file or stdin into memory before scanning. C reads through a `FILE*` stream; large CSSCPA inputs should be benchmarked before treating the eager path as final.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
