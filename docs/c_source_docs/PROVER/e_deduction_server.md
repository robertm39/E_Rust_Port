<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_deduction_server

## Source Files

- [PROVER/e_deduction_server.c](../../../eprover/PROVER/e_deduction_server.c)

## Purpose

Implementation for the deduction server executable which starts the server with the params given. the GNU Lesser General Public License.

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
- `<cco_einteractive_mode.h>`
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
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_deduction_server.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 312 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Implementation for the deduction server executable which starts the server with the params given. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/e_deduction_server.rs` and `src/bin/e_deduction_server.rs` port the standalone executable wrapper. The Rust wrapper preserves exact C-shaped full help text, the C option surface, default prover `eprover`, default 30-second total wall-clock limit, `dummy` batch category, desired proof output, first positional argument as prover, ignored extra positional arguments, the C no-port stdout-mode-not-implemented message, TCP-string mode when `-p` is present, temp-file-backed `RUN` subprocess execution through the ported batch/process-control backend, captured `RUN` global/stdout side-channel output, and the C `OutClose(GlobalOut)` final flush/error check on the successful execution path.
- The Rust TCP server starts one detached worker per accepted client. Each worker creates fresh term/control/runner state, and parser dialect state is thread-local, matching the isolation and simultaneous-service behavior of the C child-process snapshot without copying process-global mutable state between clients.
- A real loopback regression now sends the exact framed `RUN` command, uploaded formula and `GO\n` terminator through the executable client wrapper, then compares all four intended response frames with a live C byte capture. The same WSL experiment records the stock C process-controller PID-prefix defect that Rust deliberately does not reproduce; see [`experiment 044`](../../../experiments/2026-07-17-044-deduction-server-run-framing/FINDINGS.md).
- The corresponding historical cross-unit status and compatibility notes live in [`../../e-port-history.md`](../../e-port-history.md) under `E Server Sessions`.

### Change Later

- The usage string and option help say stdin/stdout will be used when `-p` is absent, but the C path calls `StartDeductionServer(spec, ctrl, server_lib, stdout, -1)`, prints `e_deduction_server: Server mode not implemented yet for stdout`, and exits without processing commands. Rust preserves this no-port message for the executable while keeping its reusable text-session helper internal; a cleaned CLI should either implement stdin/stdout mode deliberately or require `-p`.
- The executable assigns `total_wtc_limit = 30` before option parsing, so `-w 0` later overwrites the default and disables the fallback. A future configuration API should distinguish omitted limits from explicit zero.
- `outname` is passed to `OpenGlobalOut(outname)` but no command-line option sets it; `app_encode` is file-global and unused; `OPT_PRINT_STATISTICS` remains in the enum without an option-table entry. These look like stale or anticipatory C surfaces that should not be reproduced beyond observable compatibility.
- The first remaining argument is treated as the prover executable and later positional arguments are ignored, despite the help text saying `[files]`. A stricter Rust CLI should wait until drop-in compatibility tests cover this behavior.
- The TCP path forks once per accepted client. That isolates each client's uploaded axiom sets and signature mutations by process snapshot, while stdout from multiple children may interleave. Rust uses detached client threads with per-thread parser state and locked whole-report stdout writes; this deliberately gives each emitted RUN report a stronger local write boundary without changing its socket protocol. Preserve the worker-local state boundary if later process or async implementations replace these threads.
- `process_options()` exits directly for help/version before `main()` opens and later closes `GlobalOut`, while successful execution reaches `OutClose`. Keep that split visible in the compatibility wrapper; a cleaned server API should make output ownership and shutdown explicit.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
