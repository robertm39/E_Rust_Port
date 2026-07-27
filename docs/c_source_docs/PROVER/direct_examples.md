<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / direct_examples

## Source Files

- [PROVER/direct_examples.c](../../../eprover/PROVER/direct_examples.c)

## Purpose

Generate examples directly from a protocol file. the GNU Lesser General Public License. <1> Fri Jul 23 17:46:15 MET DST 1999 New

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

- `<ccl_clausesets.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<cio_tempfile.h>`
- `<e_version.h>`
- `<pcl_analysis.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/direct_examples.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 258 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Generate examples directly from a protocol file. the GNU Lesser General Public License. <1> Fri Jul 23 17:46:15 MET DST 1999 New
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/direct_examples.rs` and `src/bin/direct_examples.rs` port the standalone executable over the shared Rust PCL protocol and analysis modules used by `ekb_ginsert`.
- The Rust wrapper preserves the C command-line surface, including `-V`/`--version`, optional `--verbose`, `-o` output redirection including `-o -`, C-shaped file-open and output-close diagnostics, `InputOpen`'s pre-open regular-file `stat` boundary, default stdin input through `-`, negative-example count/proportion options, and the typo-preserving negative-proportion diagnostic.
- The executable parses each input as TPTP-format PCL with shared external variable-name mapping for compressed clause input, strips FOF steps, resets tree data, marks proof clauses, computes proof distance/reference data, selects examples, then prints `% Axioms:` followed by initial clauses, a standalone `.`, and `% Examples:` followed by selected training examples.
- Archived-C comparison covers help/version, the original stdin workload, a 12-step branching protocol, and an isolated missing-input case; the expanded 14-case learning-tool report has no mismatches.

### Change Later

- `main()` sets `ClausesHaveLocalVariables = false` before parsing so compressed PCL input can share name-to-variable mappings. Rust now expresses this through explicit `PclStepParseOptions`/`ClauseParseOptions` and has compatibility coverage for reused names; keep the C global switch documented as a candidate for cleanup after executable parity is locked down.
- C calls both `GlobalOut = OutOpen(outname)` and `OpenGlobalOut(outname)` before parsing. Rust uses one explicit output writer while preserving the important early create/truncate side effect; decide later whether the double-open is observable on any supported platform.
- Negative-example selection uses `proof_steps ? neg_proportion*proof_steps : neg_examples`, with C floating-point-to-`long` truncation and no clamp. Keep this for compatibility, but a cleaned API should use explicit positive and negative selection limits.
<!-- END MANUAL REVIEW: c_source_docs -->
