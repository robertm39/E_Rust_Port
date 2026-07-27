<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_ltb_runner

## Source Files

- [PROVER/e_ltb_runner.c](../../../eprover/PROVER/e_ltb_runner.c)

## Purpose

Hack for the LTB category of CASC-2012 (rehacked for later versions) - parse an LTB spec file, and run E on the various problems. the GNU Lesser General Public License. <1> Mon Jun 28 02:15:05 CEST 2010

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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_ltb_runner.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 423 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Hack for the LTB category of CASC-2012 (rehacked for later versions) - parse an LTB spec file, and run E on the various problems. the GNU Lesser General Public License. <1> Mon Jun 28 02:15:05 CEST 2010
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/e_ltb_runner.rs` and `src/bin/e_ltb_runner.rs` port the standalone runner wrapper over the Rust batch backend, including global output redirection through `-o`, the `-o -` stdout route, C-shaped `OutOpen` diagnostics for configured output files, the C ordering where configured output is opened before positional usage validation, and the C `OutClose(GlobalOut)` final flush/error check on the execution path.

### Change Later

- `e_ltb_runner` parses `division.category.training_data`, but the paired batch-spec printer writes `division.category.training_directory`. Official JJT/VBT parser fixtures and an older HOL rejection fixture now pin that current-C mismatch. Rust preserves it for drop-in compatibility; any future round-trip normalization is a product extension and must retain an explicit legacy mode.
- The optional second positional argument is honored here as the prover executable path, unlike `e_stratpar` where the same-looking argument is ignored. Keep those executable-specific differences visible when common runner option handling is introduced.
- C opens `GlobalOut` after option parsing and before validating the positional argument count, so `-o file` can create or truncate the output file even when the command later reports a usage error, while `-o -` routes to stdout instead of a literal file. Rust now preserves those side effects in the compatibility wrapper; a future cleaned CLI should avoid them only outside the drop-in mode.
- The help banner calls the first positional argument `[Batchfile] [PATH_TO_EPROVER]`, while the fatal usage diagnostic calls the same surface `<spec> [<path-to-eprover>]`. Rust preserves the mismatch for drop-in behavior; a cleaned CLI should use one spelling once byte-compatible help output is no longer required.
- Several option descriptions are historical competition text, including the duplicate-word interactive description and "very specific hack" variant wording. Rust preserves them for visible CLI compatibility; a cleaned help path should separate modern user-facing descriptions from the legacy drop-in text.
- A global wall-clock limit from `-w/--wtc-limit` is copied into a parsed spec only when the spec omits `limit.time.overall.wc`; the per-problem limit is still required unless one of those total limits is positive. Later configuration code should represent that precedence explicitly instead of mutating parsed specs in place.
- Runner state is stored in process globals such as `outname`, `outdir`, `total_wtc_limit`, `interactive`, `use_variants`, and `provers`. A future Rust runner should make those fields explicit while preserving option timing and output behavior.
- `process_options()` exits directly for help/version before `main()` opens and later closes `GlobalOut`, while ordinary execution reports close-time output errors through `OutClose`. Keep that split visible in compatibility wrappers; a cleaned API should expose explicit output ownership instead of inheriting executable control flow.
<!-- END MANUAL REVIEW: c_source_docs -->
