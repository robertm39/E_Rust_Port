<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_server

## Source Files

- [PROVER/e_server.c](../../../eprover/PROVER/e_server.c)

## Purpose

Parse a problem specification and a filter setup, and offer deduction in the specification as a service via a TCP port. the GNU Lesser General Public License. <1> Mon Feb 21 13:24:04 CET 2011

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

- `main`: Main function of the program.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_relevance.h>`
- `<ccl_sine.h>`
- `<cco_batch_spec.h>`
- `<cio_commandline.h>`
- `<cio_network.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<clb_defines.h>`
- `<e_version.h>`
- `<netinet/in.h>`
- `<sys/select.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_server.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 486 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Server executable entry point; network/session behavior is user-visible for remote proving workflows.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/server.rs` and `src/bin/umlaut-server.rs` port the standalone executable wrapper. The Rust wrapper preserves exact C-shaped full help text with the legacy footer, the C option surface, default prover `eprover`, default service port `3666`, output-file redirection including `-o -`, C-shaped output-file open diagnostics, output-file creation before missing-domain usage errors, custom/default ax-filter parsing, domain-spec parsing through the ported structured-FOF include loader, C-shaped reset of the shared boundary after distribution initialization, the observed TCP-string response loop that prints each message and replies `wait` then `ready`, one `Main loop` marker before each logical blocking wait, the stable `Read error` and `Connection closed` lines, and the C one-active-connection loop behavior that closes an accepted second socket while the first remains active.
- The executable's unchecked `accept` result is also compatibility-visible: a failure with no active client prints `Accepted -1`, while a failure with an active client is silently passed to `close(-1)`. Focused regressions inject both paths. The comparison harness normalizes only nonnegative runtime descriptors in successful `Accepted <descriptor>` lines and intentionally leaves `Accepted -1` unchanged. The source audit and environment limitation are recorded in [`../../../experiments/2026-07-16-025-e-server-loop-compatibility/FINDINGS.md`](../../../experiments/2026-07-16-025-e-server-loop-compatibility/FINDINGS.md).
- The corresponding historical cross-unit status and compatibility notes live in [`../../e-port-history.md`](../../e-port-history.md) under `E Server Sessions`.

### Change Later

- The service loop parses the domain spec and filter set but never uses them to run the prover. It only prints `Received: ...` and sends `wait` then `ready` for every TCP string; no `result` message is produced for the legacy `e_client` protocol. Keep this for drop-in compatibility, but decide later whether to implement the intended service or retire this placeholder.
- `--tptp-in`, `--tptp-format`, `--tptp2-in`, and `--tptp2-format` are advertised in the option table, but `process_options` has no switch cases for their option codes. In normal C builds they leave the default TSTP parser unchanged. Rust preserves this no-op; a cleaned CLI should either implement the aliases or remove them.
- `--output-file` affects `GlobalOut` output from startup parsing, while the main loop uses `printf` directly to stdout for `Main loop`, `Accepted`, `Received`, and connection diagnostics. Preserve the split for compatibility, but make output routing explicit in a cleaned server API.
- `OutClose(GlobalOut)` is after the infinite service loop, so normal operation never closes redirected startup output or reports close-time output errors. Rust flushes startup parse output before serving; a future server owner should define shutdown and flush behavior deliberately.
- C prints the raw accepted descriptor in `Accepted %d`; this value necessarily varies by process and platform, so compatibility comparisons should normalize only successful nonnegative descriptors. A cleaned server API should replace both the raw descriptor and the preserved `Accepted -1` failed-accept quirk with stable structured diagnostics.
- `app_encode` and `OPT_PRINT_STATISTICS` are present but unused in this file. Do not reproduce them in cleaned Rust APIs unless another compatibility path exposes them.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
