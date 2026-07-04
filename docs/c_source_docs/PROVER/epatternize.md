<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / epatternize

## Source Files

- [PROVER/epatternize.c](../../../eprover/PROVER/epatternize.c)

## Purpose

Read a logic file and convert it to pattern form. the GNU Lesser General Public License.

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

- `CLState_p process_options(int argc, char* argv[], SpecLimits_p limits)`
- `char* parse_feature_line(Scanner_p in, SpecFeature_p features)`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `main`: The main function and entry point of the program.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.
- `print_help`: Print the help text.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_unfold_defs.h>`
- `<cco_sine.h>`
- `<che_clausesetfeatures.h>`
- `<che_hcb.h>`
- `<che_rawspecfeatures.h>`
- `<che_specsigfeatures.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cle_clauseenc.h>`
- `<cle_patterns.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `STACK_SIZE`
- `term`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/epatternize.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 864 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Read a logic file and convert it to pattern form. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `src/prover/epatternize.rs` and `src/bin/epatternize.rs` port the standalone executable wrapper. The Rust path preserves the long-only `--version`, default stdin input, LOP/TPTP/TSTP parse selectors, TPTP3 aliases, output-file routing including `-o -`, C-shaped file-open and output-close diagnostics, exact C mask-length checks, explicit `--sine` filtering, compatibility acceptance of parsed classification/preprocessing options, represented formula-owner parsing/CNF for currently supported parser fragments, pattern computation, flat clause-list encoding, and pattern-term output.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `print_help()` says `Usage: classify_problem [options] [files]` even though the executable name is `epatternize`. Rust preserves this visible typo; a cleaned support-tool help layer should use the actual program name once compatibility mode is not required.
- `process_options()` sets `parse_features`, `raw_classify`, `specsig_classify`, `tptp_header`, `mask`, `raw_mask`, and many `SpecLimits` fields, but `main()` never reads them before patternizing formulas and clauses. These classification remnants should either be removed or moved to a real classifier interface after drop-in compatibility is secured.
- `process_options()` also sets `no_preproc`, equation-unfolding limits, `FormulaDefLimit`, and `miniscope_limit`; the visible main path ignores the clause-preprocessing flags and always calls formula conjecture preprocessing followed by `FormulaSetCNF2`, using the formula-CNF limits. Rust mirrors that phase split for supported parser fragments. The option semantics should be made explicit if a modernized patternizer exposes preprocessing controls.
- TPTP/TSTP output-format options mutate global formula/equation printing state, but the executable prints only pattern terms after flat clause encoding. Keep the no-op user-visible behavior for compatibility, then remove the confusing print-format surface in a cleaned CLI.
- `PatternDefaultSubstAlloc()` is called once per file and backtracked before each clause, while `FlatEncodeClauseListRep()` can depend on special signature symbols for the encoded clause term. A future shared pattern/encoding API should make the required special-symbol initialization explicit instead of relying on mutable global signature state.
- `main()` calls `OpenGlobalOut(outname)` before it inserts the default `-` input and before any scanner is opened, so the requested output file may be created or truncated even when input opening or parsing later fails. Rust preserves this ordering for compatibility; a future cleaned interface could offer atomic or transactional output behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
