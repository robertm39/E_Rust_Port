<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / epcllemma

## Source Files

- [PROVER/epcllemma.c](../../../eprover/PROVER/epcllemma.c)

## Purpose

Read a PCL protocol and suggest certain clauses as lemmas. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LemmaAlgorithm`
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
- `<pcl_lemmas.h>`
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

Source files reviewed: `PROVER/epcllemma.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 664 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a PCL protocol and suggest certain clauses as lemmas. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/epcllemma.rs` and `src/bin/epcllemma.rs` port the standalone `epcllemma` executable over the existing Rust PCL2 lemma-selection core. The port covers `-h`/`--help`, long-only `--version`, `-v`/`--verbose`, `-o`/`--output-file` including `-o -`, `-s`/`--silent`, `-l`/`--output-level`, PCL/TPTP/TSTP/LOP output selection, iterative/recursive/flat lemma algorithms, absolute and relative lemma count/quality thresholds, lemma-quality weights, proof-tree inference weights, default stdin input through `-`, TPTP-format UPCL2 parsing with C-compatible shared external variable-name mapping for compressed clause input, strict end-of-input checks, stdout status lines, output-level-controlled lemma/full-protocol printing including empty, formula-valued, and shell-step protocols, two-line `SysError`-style file-open diagnostics, and C `OutClose` wording on final flush failure.

### Change Later

- C exposes `--version` without a `-V` shorthand. Rust preserves that option table; add a short alias only outside drop-in compatibility mode.
- `main()` disables C's global `ClausesHaveLocalVariables` before full PCL parsing, so compressed clauses share external variable names across the protocol. Rust preserves this through explicit `PclStepParseOptions`/`ClauseParseOptions`; keep future parser entry points on explicit configuration rather than hidden process state.
- C prints the `% Selecting at most ...` and `% Minimum lemma quality ...` status lines with `printf`, so they always go to stdout even when `-o` redirects lemma output. Rust preserves this visible split; a cleaner UI could route all non-diagnostic output through one selected stream after compatibility baselines exist.
- `OpenGlobalOut(outname)` runs before inserting the default `-` input and before parsing, so `-o` can create/truncate the lemma-output path before later input failures while `-o -` leaves lemma output on stdout with the status lines. Rust preserves this ordering with a local output owner; transactional output should be limited to a cleaned non-compatibility mode.
- `OPT_LOP_PRINT` lacks a `break` and falls through into `OPT_ITERATIVE_LEMMAS`, so `--lop-out` also resets the algorithm to iterative unless a later option changes it again. Rust preserves that order-sensitive behavior.
- `--no-reference-weights` is documented as clearing all reference weights, but C assigns `pas_simpl_w` twice and leaves `act_simpl_w` unchanged. Rust preserves the effective assignments; revisit only with lemma-selection trace comparisons.
- The default relative lemma limit uses `PCLProtStepNo(prot) * max_lemmas_rel + 0.99` and then stores the result in `long`; this can still truncate to zero for very small protocols. Rust preserves the numeric behavior; a clearer UI could expose explicit rounding only outside compatibility mode.
- `print_help(FILE* out)` prints the option table to `stdout` instead of `out`, and the footer uses the old 2003-2005 support-tool copyright block. Rust keeps the visible executable text but not the internal helper-output bug.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
