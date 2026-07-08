<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / classify_problem

## Source Files

- [PROVER/classify_problem.c](../../../eprover/PROVER/classify_problem.c)

## Purpose

Read a specification and print classification and feature vector. the GNU Lesser General Public License. <1> Sat Dec 12 22:39:18 MET 1998 New

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

- `parse_raw_feature_line`: Parse a single specification features line of the form <name> : ( <features> ) : <class> where <name> and <class> can be parsed by ParsePlainFileName(). <name> is returned, <class> is ignored, and <features> is stored in features.
- `parse_feature_line`: Parse a single specification features line of the form <name> : ( <features> ) : <class> where <name> and <class> can be parsed by ParsePlainFileName(). <name> is returned, <class> is ignored, and <features> is stored in features.
- `process_raw_feature_files`: Given a file of pre-evaluated raw feature-lines, read it and add a new symbolic class name based on the given class limits for the features.
- `process_feature_files`: Given a file of pre-evaluated feature-lines, read it and add a new symbolic class name based on the given class limits for the features.
- `print_tptp_header`: Generate a TPTP style header for the parsed problems. This is a service for Geoff back in the ancient times when his code could not handle real men's problems...
- `do_raw_classification`: Perform a very high-level classification of the unprocessed problem based (preliminary) on the following 3 features: Number of sentences (fof and cnf) Rough term size (ClauseStandardWeight for cnf, TermStandardWeight for fof). Number of symbols in the signature.
- `main`: The main function and entry point of the program.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.
- `print_help`: Print the help text.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_unfold_defs.h>`
- `<cco_sine.h>`
- `<che_clausesetfeatures.h>`
- `<che_new_autoschedule.h>`
- `<che_rawspecfeatures.h>`
- `<che_specsigfeatures.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `STACK_SIZE`
- `criteria`
- `term`

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

Source files reviewed: `PROVER/classify_problem.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 1211 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 9 structured function-comment blocks.
- Read a specification and print classification and feature vector. the GNU Lesser General Public License. <1> Sat Dec 12 22:39:18 MET 1998 New
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

### Compatibility Notes

- The executable has two major execution paths. With `--parse-features`, it consumes precomputed feature lines and recomputes only the symbolic class. Without `--parse-features`, it parses real formula/clause inputs, optionally applies SInE, clausifies, preprocesses, and computes feature vectors from the resulting proof state.
- `parse_feature_line()` and `parse_raw_feature_line()` parse `<name> : <features> : <old-class>` and intentionally ignore the old class after it has supplied any structural fields needed by the feature parser.
- `process_feature_files()` calls `SpecFeaturesAddEval()` after parsing and then prints the feature vector and `SpecTypePrint()` classification. `process_raw_feature_files()` calls `RawSpecFeaturesClassify()` and prints only `RawSpecFeaturesPrint()`, whose output already includes the raw class.
- `--parse-features` is described as conflicting with `--generate-tptp-header`, but the C control flow does not reject the combination; the parse-feature branch ignores header generation.
- The real-input `--raw-class` branch still runs the normal real-input parser and `ProofStateSinE()` before computing `RawSpecFeaturesCompute()`, but stops before formula CNF, clause preprocessing, spec-signature output, or TPTP header generation.
- The non-raw real-input branch computes raw features before formula CNF/preprocessing, then copies the raw order and formula-definition fields back into the final `SpecFeatureCell` after `SpecFeaturesCompute()`.
- In the non-raw real-input branch, formula conjecture preprocessing and `FormulaSetCNF2()` run unconditionally before the `if(!no_preproc)` clause-preprocessing gate. Thus `--no-preprocessing` suppresses later clause cleanup but does not suppress formula conjecture normalization or formula CNF conversion in C. This caller also does not invoke `ClauseSetUnfoldEqDefNormalize()` outside that gate, so the option suppresses clause-level equality-definition unfolding in `classify_problem` even though other proof-state preprocessing callers can apply that bridge separately.
- `--merged-classification=N` wins over `--raw-class` when `N != -1`: it prints the raw classification prefix plus a child-computed CNF classification string, and uses an all-hyphen CNF class if the child cannot write the full fixed-width buffer.
- `src/prover/classify_problem.rs` and `src/bin/classify_problem.rs` port the executable wrapper over the feature-line consumers and supported real-input parser fragments. The Rust wrapper preserves default stdin through `-`, output-file routing including `-o -`, two-line `SysError`-style file-open diagnostics for feature-line and real-input scanners, C `OutClose` wording on final flush failure, no-partial-output behavior for malformed feature lines, parser-owned supported real-input formulas in `f_axioms`, the represented formula-owner `FormulaSetPreprocConjectures`/`FormulaSetCNF2` stage before clause preprocessing, the C caller's `--no-preprocessing` boundary for clause cleanup and equality-definition unfolding, explicit parsed-dialect threading into real-input raw/CNF/spec classification, and hidden-child merged classification for positive standalone timeouts.

### Change Later

- `raw_mask` is initialized to `"aaaaaaaaaa"` even though `--raw-mask` validation rejects strings shorter than 11 characters. Preserve the initialized default for compatibility, then choose a single documented mask width in a cleanup mode.
- The option table includes `--old-cnf`, but `process_options()` has no `OPT_DEF_CNF_OLD` case. Release builds effectively ignore it, while assertion-enabled builds can hit the default assertion. Treat this as a compatibility quirk; a cleaned CLI should remove the dead switch or wire it to a deliberate CNF mode after parser parity is secured.
- `process_options()` mutates many globals that are used only by the real-input branch. A Rust cleanup should keep parse-feature options separate from clausification/preprocessing options once drop-in behavior is covered.
- `OpenGlobalOut(outname)` runs before defaulting missing input to `-` and before feature-line or real-input scanners are opened, so `-o` can create or truncate the output path before later input failures while `-o -` remains stdout. Rust preserves that side effect through an explicit output owner; transactional output belongs outside drop-in mode.
- `do_raw_classification()` depends on global `raw_mask` even though the mask is otherwise parsed as command-line state. A cleaned API should pass all classification inputs explicitly after the drop-in executable behavior is covered.
- The real-input path shares one `skip_includes` tree across all files in `main()`, but this checkout's visible formula parser does not populate that tree before `ScannerParseInclude` consults it. Preserve the resulting repeated-include behavior for drop-in compatibility; if a cleaned API wants cross-file include suppression, make it an explicit seeded caller policy rather than an implicit side effect.
- `FormulaAndClauseSetParse()` owns TSTP wrapper problem-type setup for real inputs: `thf(...)` records select higher-order parsing at the wrapper boundary and mixed FO/HO records are rejected through the global problem type. Rust preserves parser-side setup/rejection but threads the returned parsed dialect into later real-input classification; a cleaned API should continue passing dialect state explicitly instead of relying on hidden process-global mutation.
- `RawSpecFeaturesClassify()` and `SpecTypePrint()` read the same process-global `problemType` residue after parsing real inputs. Rust now passes the parsed problem type explicitly for real-input classification while keeping feature-line reclassification on the C-shaped global helper; a cleaned classifier should make this boundary visible in the API.
- The real-input option surface mixes formula-CNF knobs (`--definitional-cnf`, `--miniscope-limit`), the clause-level `--no-preprocessing` gate, and equation-definition unfolding controls, but C applies them in separate phases with different bypass rules and caller-specific equality-definition unfolding boundaries. Rust now populates represented formula owners for supported parser fragments and runs the owner CNF hook before preserving the `classify_problem` clause gate; full `FormulaAndClauseSetParse` coverage remains parser work. A cleaned classifier should expose the phases explicitly after exact formula-owner parity is in place.
- `print_tptp_header()` calls `ClauseSetTPTPDepthInfoAdd()` to fill local depth variables, but the printed depth fields come from the already-computed `SpecFeatureCell`. Keep the visible output for compatibility, then remove or explain the unused local computation in a cleanup pass.
- `ClausifyAndClassifyWTimeout()` hard-codes POSIX `pipe()`, `fork()`, and `RLIMIT_CPU` around a deterministic classification computation after parsing and SInE filtering have already mutated the proof state. Rust uses an explicit hidden re-exec child for standalone positive timeouts, reparsing file inputs and piping buffered stdin inputs; a cleaned implementation should make timeout/process isolation and inherited-state requirements explicit instead of burying them inside the feature classifier.
- `OpenGlobalOut(outname)` runs before either feature-line or real-input parsing, so output paths can be created or truncated even if later input parsing fails. Rust preserves this order; a cleanup mode could stage output in memory or a temporary file before replacing the destination.
<!-- END MANUAL REVIEW: c_source_docs -->
