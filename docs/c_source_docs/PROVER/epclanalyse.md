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

- `src/prover/epclanalyse.rs` and `src/bin/epclanalyse.rs` port the standalone `epclanalyse` executable over the existing Rust PCL2 full-protocol owner and property-analysis functions. The port covers exact C-shaped full help text with the legacy support-tool footer, long-only `--version`, `-v`/`--verbose`, `-o`/`--output-file` including `-o -`, `-s`/`--silent`, default stdin input through `-`, TPTP-format PCL parsing with C-compatible shared external variable-name mapping for compressed clause input, strict end-of-input checking, property-statistic aggregation through `PCLProtPropAnalyse`-style logic, C-shaped summary/representative-step output, safe empty/formula-only representative handling, two-line `SysError`-style file-open diagnostics, and C `OutClose` wording on final flush failure. Permanent differential cases include a formula-plus-empty-clause boundary that reaches every zero-denominator average while retaining a valid C clause representative, plus an isolated missing-file case; only the runtime-specific non-finite average fields and known not-found suffix are normalized.

### Change Later

- Unlike nearby tools such as `epclextract`, C `epclanalyse` defines `--version` without a `-V` shorthand. Rust preserves that option table; add `-V` only behind an explicit compatibility decision.
- `main()` disables C's global `ClausesHaveLocalVariables` before full PCL parsing, so compressed clauses share external variable names across the protocol. Rust preserves this through explicit `PclStepParseOptions`/`ClauseParseOptions`; keep future parser entry points on explicit configuration rather than hidden process state.
- `--silent` sets C's global `OutputLevel` to `0`, but the property-summary output is not level-gated. Rust preserves the global output-level side effect while keeping summary rendering unconditional; a later cleanup can remove or document it if no shared output layer consumes the side effect.
- `OpenGlobalOut(outname)` runs before inserting the default `-` input and before input scanning, so `-o` can create/truncate an output path before later input failures while `-o -` remains stdout. Rust preserves the side effect through an explicit output owner; transactional output belongs outside the drop-in replacement mode.
- The C help text uses a legacy 2002-2009 support-tool footer and obsolete URL instead of the shared modern `E_FOOTER`. Rust preserves that visible text for this executable; consider moving old support-tool footers behind a shared compatibility helper later.
- `epclanalyse` inherits `PCLProtPropDataPrint`'s zero-denominator average formulas and unconditional representative/metric printing. For an empty protocol, every `PCLProtFindMaxStep` result is null and the first `PCLStepPrint` asserts or dereferences it. For a formula-only protocol, the representative searches return a formula step, then the heaviest section reads its `logic.clause` union member and passes that invalid pointer to `ClausePropInfoPrint`. The archived release tool terminates with `SIGSEGV` on both focused corpora. Rust deliberately preserves the visible arithmetic and formula representative shapes while making both paths total; crashing or reproducing the union read is not a compatibility requirement. After drop-in compatibility is secured, the C path should report unavailable representatives explicitly.
- Even the small non-empty comparison fixture reaches a zero denominator for one metric: glibc spells the resulting value `-nan`, while Rust's formatter spells it `NaN`, and legacy Microsoft runtimes can use `-1.#IND00`. The comparison harness canonicalizes only the known `epclanalyse` average fields; unrelated non-finite text remains strict. A cleaned statistics renderer should print an explicit unavailable marker instead of exposing runtime-specific NaN text.
- The executable opens named inputs through C's `InputOpen` boundary before scanner construction, so missing paths report `Cannot stat file ...` and still retain the second `epclanalyse:` system-error line. Stdin scanners use C's diagnostic source name `<stdin>`. The current five-case permanent comparison matrix is exact.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
