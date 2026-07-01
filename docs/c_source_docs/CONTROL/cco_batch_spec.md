<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_batch_spec

## Source Files

- [CONTROL/cco_batch_spec.h](../../../eprover/CONTROL/cco_batch_spec.h)
- [CONTROL/cco_batch_spec.c](../../../eprover/CONTROL/cco_batch_spec.c)

## Purpose

Data types and code for dealing with CASC-2010-2019 LTB batch specifications. It's unclear if this will ever be useful for other applications... the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `BOOutputType`
- `BatchSpecCell`
- `BatchSpec_p`

### Macros And Constants

- `BatchSpecCellAlloc()`
- `BatchSpecCellFree(junk)`
- `BatchSpecProblemNo(spec)`
- `CCO_BATCH_SPEC`

### Globals

- None found in the source scan.

### Exported Functions

- `BatchSpec_p BatchSpecAlloc(char* executable, IOFormat format)`
- `BatchSpec_p BatchSpecParse(Scanner_p in, char* executable, char* category, char* train_dir, IOFormat format)`
- `bool BatchProcessFile(BatchSpec_p spec, long wct_limit, StructFOFSpec_p ctrl, char* default_dir, char* source, char* dest)`
- `bool BatchProcessProblem(BatchSpec_p spec, long wct_limit, StructFOFSpec_p ctrl, char* jobname, ClauseSet_p cset, FormulaSet_p fset, FILE* out, int sock_fd, bool interactive)`
- `long BatchProcessProblems(BatchSpec_p spec, StructFOFSpec_p ctrl, long total_wtc_limit, char* default_dir, char* dest_dir)`
- `long BatchStructFOFSpecInit(BatchSpec_p spec, StructFOFSpec_p ctrl, char *default_dir)`
- `void BatchProcessInteractive(BatchSpec_p spec, StructFOFSpec_p ctrl, FILE* fp)`
- `void BatchProcessVariants(BatchSpec_p spec, char* variants[], char* provers[], long start, char* default_dir, char* outdir)`
- `void BatchSpecFree(BatchSpec_p spec)`
- `void BatchSpecPrint(FILE* out, BatchSpec_p spec)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `batch_create_runner`: Create a EPCtrl block associated with a running instance of E.
- `parse_op_line`: Parse an output line into batchspec
- `print_op_line`: Print an output line in spec to out
- `abstract_to_concrete`: Replace the * in an abstract name by the variant and append the ending. Ignores everything after * in name. The result is returned and must be freed by the caller.
- `concrete_batch_struct_FOF_spec_init`: Initialise a StructFOFSpecCell for the concrete problems encoded in *variant.
- `BatchSpecAlloc`: Allocate an empty, initialized batch spec file.
- `BatchSpecFree`: Free a batch spec structure with all information.
- `BatchSpecPrint`: Print a BatchSpec cell in the original form (or as close as I can make it).
- `BatchSpecParse`: Parse a batch specification file. This is somewhat wonky - the spec file syntax is not really well-defined, and what we know about them is that comments and newlines are significant for the structure. This just ignores those and hopes for the best.
- `BatchStructFOFSpecInit`: Initialize a BatchStructFOFSpecCell up to the symbol frequency.
- `StructFOFSpecAddProblem`: Add a problem as one set of clauses and formulas, each. Note that this transfers the two sets into ctrl, which is responsible for freeing.
- `StructFOFSpecBacktrackToSpec`: Backtrack the state to the spec state, i.e. backtrack the frequency count and free the extra clause sets. Also backtracks the signature to forget all new symbols.
- `StructFOFSpecGetProblem`: Given a prepared StructFOFSpec, get the clauses and formulas describing the problem.
- `BatchProcessProblem`: Given an initialized StructFOFSpecCell for Spec, parse the problem file and try to solve it. Return true if a proof has been found, false otherwise.
- `BatchProcessFile`: Given an initialized StructFOFSpecCell for Spec, parse the problem file and try to solve it. Return true if a proof has been found, false otherwise.
- `BatchProcessProblems`: Process all the problems in the StructFOFSpec structure. Return number of proofs found.
- `BatchProcessInteractive`: Perform interactive processing of problems relating to the batch processing spec in spec and the axiom sets stored in ctrl.
- `BatchProcessVariants`: Try to solve the abstract problems in spec by going through the concrete variants indicated by variants.

### Dependencies

- `"cco_batch_spec.h"`
- `"cco_gproc_ctrl.h"`
- `<ccl_formulafunc.h>`
- `<ccl_sine.h>`
- `<cco_proc_ctrl.h>`
- `<cco_sine.h>`
- `<cio_network.h>`
- `<cio_simplestuff.h>`
- `<cio_tempfile.h>`

### Compile-Time Conditions

- `CCO_BATCH_SPEC`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_batch_spec.h`, `CONTROL/cco_batch_spec.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 1365 lines, 13 scanned public declarations, 0 scanned internal function definitions, and 18 structured function-comment blocks.
- Data types and code for dealing with CASC-2010-2019 LTB batch specifications. It's unclear if this will ever be useful for other applications... the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Rust support is in `src/control/batch_spec.rs`, covering the output-type enum values, batch filter/strategy tables, owned batch-spec state, scanner-backed config/include/problem-list parsing, C-shaped printing, `BatchSpecProblemNo`, `e_ltb_runner` header parsing, include acceptance notices, abstract variant filename construction, staged `batch_create_runner` request construction over selected problem counts and temporary TSTP problem files, direct `BatchProcessProblem` execution over a temp-file/process backend, staged `BatchProcessFile` orchestration over an injected problem loader and destination writer, staged `BatchProcessVariants` orchestration over injected concrete-problem execution, and a `BatchProcCtrlRunnerSet` adapter from staged runner requests to `EPCtrlSet` child polling.
- Shared-axiom and selected-problem support used by this unit is staged in `src/control/sine.rs`, covering `StructFOFSpecAddProblem`, `StructFOFSpecBacktrackToSpec`, and `StructFOFSpecGetProblem` behavior over already-parsed clause/formula sets.
- `BatchProcessProblems` orchestration is now represented in `src/control/batch_spec.rs` with injected file-processing and clock hooks, preserving per-problem time allocation, destination-name construction, and solved-count behavior.

### Change Later

- `BatchSpecParse` comments call the spec syntax "wonky": comments and newlines are significant to the external format, but this implementation ignores them and hopes scanner token flow is enough. Preserve this while building drop-in compatibility; later, replace it with an explicit grammar only after real LTB corpus tests cover accepted legacy forms.
- The input path through `e_ltb_runner` accepts `division.category.training_data`, while `BatchSpecPrint` emits `division.category.training_directory`. Keep both spellings visible until compatibility tests decide whether the printer should remain mismatched or normalize the field name.
- `BatchSpecParse` prints `% Accepted ... for parsing` to stdout while parsing includes. That side effect belongs to parsing rather than axiom loading, and a future interface may want to move it behind a runner-level logging decision.
- The problem-list loop starts on either a slash token or the exact identifiers `Problem|Problems`, then consumes two filename-shaped token streams. This is path-shape compatibility rather than a robust section grammar.
- `abstract_to_concrete` ignores everything after the first `*` in an abstract filename. If future variant specs need suffix preservation, change this only behind compatibility tests because the truncation is documented in the C function comment.
- Training-directory values are parsed with the normal scanner's continuous-token behavior, so comment delimiters still have scanner semantics inside unquoted paths. Consider quoted or explicitly delimited fields only after matching existing LTB files.
- This C file implements several `StructFOFSpec*` helpers declared in `cco_sine.h`, which obscures ownership boundaries between batch execution and SInE selection. Rust keeps those helpers in `control::sine`; if future callers need a different module split, prefer a caller-driven boundary rather than mirroring the C file layout.
- `BatchProcessProblems` computes proportional limits as `rest/(sp-i)+1`, so it deliberately rounds upward and may yield negative per-problem limits if the total wall-clock budget is already exhausted. Keep this visible when wiring real timeout/resource-limit behavior.
- Destination paths are formed with raw string concatenation: `dest_dir`, then `/`, then the spec destination. Empty `dest_dir` becomes `/dest`, and a trailing slash becomes a double slash. Rust preserves this for compatibility; a later cleaned API can use path-aware joining only behind tests.
- `batch_create_runner` interleaves progress logging, selected-problem construction, temporary-file allocation and writing, process-name formatting, and `ECtrlCreateGeneric` spawning. Rust stages that as a request object first; once real runner traces exist, split the C helper's responsibilities into selection, problem rendering, temp-file ownership, and process launch.
- `batch_create_runner` uses a stack `char name[320]` and ignores `AxFilterPrintBuf`'s false return when the printed filter does not fit. Rust exposes the fixed-size boundary as an explicit error; after compatibility is secure, prefer dynamically owned process names or a documented truncation policy.
- The C loop indexes destination files by source-file stack length. Rust validates manually constructed mismatched specs and reports an interface diagnostic; parsed specs still preserve the C paired-push invariant.
- `BatchProcessProblem` leaves the local `handle` pointer uninitialized if the wall-clock limit has already expired before the outer loop runs, then tests it after the loop. Rust treats that expired-before-spawn path as GaveUp and backtracks, documenting this as accidental undefined behavior rather than matching it literally.
- `BatchProcessProblem` computes child limits as `MIN((wct_limit+1)/2, wct_limit-used)`, so exhausted or negative per-problem budgets can flow through as zero or negative child CPU budgets. Preserve the formula until real runner compatibility tests decide whether a cleaned scheduler should clamp it.
- `BatchProcessProblem` mixes solver status formatting with global output, optional destination file output, optional socket output, and interactive proof echoing. Rust stages separate global/external writers and leaves socket mirroring pending; later cleanup should separate status generation from transport once output compatibility is pinned down.
- `BatchProcessFile` forces TSTP scanner format for all problem files rather than deriving it from the batch specification or source extension. Rust carries this as an explicit loader request field.
- `BatchProcessFile` parses the source before opening the destination with `SecureFOpen(dest, "w")`, which means parse failures should not create or truncate the destination output. Preserve that ordering when replacing the staged destination writer with real filesystem opening.
- `BatchProcessFile` has commented-out Started/Ended status output but still calls `fflush(stdout)` after scanner creation. Rust does not model that isolated flush; revisit only if output-trace compatibility requires it.
- `BatchProcessVariants` mutates `spec->executable` to each variant's prover and restores it after the round. Rust carries the prover explicitly in the staged job; final wiring should decide whether observable compatibility requires temporary mutation.
- The CASC-28/J10 path in `BatchProcessVariants` allocates, initializes, and frees a fresh `StructFOFSpec` for every concrete problem, leaving the older round-shared initialization disabled. Preserve this per-problem reload until real variant traces show whether shared axioms can be reintroduced safely.
- `BatchProcessVariants` keeps iterating later variants after all abstract problems are solved, printing "already solved" messages instead of breaking out early. Rust preserves that staged loop shape; a future scheduler can break early only behind compatibility tests.
- `BatchProcessProblem` uses `EPCtrlSetGetResult(procs, true)` and later `EPCtrlSetFree(procs, true)`, coupling solver polling, no-proof child deletion, and temporary-file cleanup. Rust keeps that visible in `BatchProcCtrlRunnerSet`; later cleanup can separate temporary-file ownership from process polling after compatibility traces are available.
- `batch_create_runner` calls `SigPrintTypeDeclsTSTP` for the whole signature before printing the filtered selected clauses/formulas, so child problems may contain declarations unrelated to the selected problem. Rust preserves this broad declaration output; consider selective declarations only after typed LTB compatibility is covered.
- `BatchProcessProblem` checks `EPCtrlSetCardinality(procs)` before launching more children, so failed-child deletion during polling reopens capacity. Rust models this as a backend `active_count()` query; keep that live-pool behavior distinct from historical spawned-runner records.
<!-- END MANUAL REVIEW: c_source_docs -->
