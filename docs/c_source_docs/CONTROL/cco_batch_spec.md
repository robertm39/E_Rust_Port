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
<!-- END MANUAL REVIEW: c_source_docs -->
