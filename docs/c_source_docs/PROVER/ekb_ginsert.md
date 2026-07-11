<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / ekb_ginsert

## Source Files

- [PROVER/ekb_ginsert.c](../../../eprover/PROVER/ekb_ginsert.c)

## Purpose

Generate new training examples from protocols and insert them into a knowledge base. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- None found in the source scan.

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
- `<cio_fileops.h>`
- `<cio_signals.h>`
- `<cio_tempfile.h>`
- `<cle_kbinsert.h>`
- `<e_version.h>`
- `<pcl_analysis.h>`

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

Source files reviewed: `PROVER/ekb_ginsert.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 367 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Generate new training examples from protocols and insert them into a knowledge base. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- The executable reads `problems` and `description` first, determines the generated example name, then writes the generated `FILES/<name>` payload before it parses `signature` and `clausepatterns`.
- Without `--name`, the name is taken from the first remaining input argument before the no-argument stdin default is inserted. If stdin is the effective input, the fallback name is `__problem__<proof_examples->count+1>`.
- All input protocol files are accumulated into one `PCLProt` and inserted as one KB example. The C usage string says `[name]`, but the remaining arguments are treated as protocol input files.
- Negative examples use `kb_desc->neg_proportion * proof_steps` assigned to a `long`, so fractional results are truncated by the C conversion. Failed or proofless runs use `kb_desc->fail_neg_examples`.
- The generated file format is visible compatibility surface: a `% Axioms:` section printed in LOP format, one standalone `.`, then a `% Examples:` section printed by `PCLProtPrintExamples`.
- The C code sets `ClausesHaveLocalVariables = false` before parsing protocols so variable names map consistently across this generated example workflow. Rust now preserves this with explicit `PclStepParseOptions`/`ClauseParseOptions`.
- `main()` sets the process-global `OutputLevel` to `0` before option processing even though this executable exposes no silent/output-level option. Rust preserves the hidden startup side effect for in-process compatibility.

### Change Later

- Make generation and integration transactional. The C flow can leave a generated `FILES/<name>` without matching `problems`/`clausepatterns` metadata if a later parse or write step fails.
- Consider whether multi-file input should stay a single generated example or become an explicit batch mode after drop-in compatibility is secured.
- Replace the implicit floating-point-to-`long` negative-example budget with a named policy that documents truncation and boundary behavior.
- Revisit the signal/temp-file setup and global `ClausesHaveLocalVariables` mutation when the Rust executable surface has a unified process-lifetime and parser-state model; the Rust port currently keeps the variable policy as explicit parser configuration.
- Replace the hidden `OutputLevel = 0` executable startup mutation with explicit local output state in any cleaned API that is not trying to be a drop-in replacement.
- The current WSL C reference aborts with glibc `double free or corruption (out)` on the comparison harness's small stdin protocol after creating partial KB output, while the ownership-safe Rust path completes. Do not reproduce the heap corruption; isolate whether the trigger is malformed legacy input or a protocol/KB ownership defect before promoting this fixture to an exact-output baseline.
<!-- END MANUAL REVIEW: c_source_docs -->
