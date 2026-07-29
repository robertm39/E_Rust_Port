<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_client

## Source Files

- [PROVER/e_client.c](../../../eprover/PROVER/e_client.c)

## Purpose

Parse a problem specification, connect to the e_server, and have it trie to solve it. the GNU Lesser General Public License. <1> Mon Feb 21 13:24:04 CET 2011

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

- `tcp_msg_wait`: Blockingly read messages off the provided socket until the expected reply has been read. Dump communication to GlobalOut.
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
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_client.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 351 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Client executable entry point; argument and protocol behavior must match the C tool.
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

### Rust Port Notes

- `src/prover/client.rs` and `src/bin/umlaut-client.rs` port the standalone client executable over the Rust TCP string-message helpers from `cio_network`, with exact C-shaped full help text pinned by a byte-for-byte unit snapshot.
- The Rust wrapper preserves the C option surface, including `-V`/`--version`, optional `--verbose`, `-o` including `-o -`, `-S`/`--server`, both `--service-port` and `--port`, default server `localhost`, default port `3666`, and the warning-but-continue behavior for ports below `IPPORT_RESERVED`.
- The executable loads all positional inputs, defaulting to `-`, concatenates them without inserted separators, preserves C-shaped `InputOpen` stat/non-regular/open diagnostics, opens the output route before loading inputs or connecting, sends `hello`, waits while echoing `% Server: ...` until `ready`, sends `add`, the problem text, and `prove`, then echoes until `result`.
- Output-file open and final output-flush failures preserve the C `OutOpen`/`OutClose` diagnostic wording.

### Change Later

- `e_client.c` targets the legacy `e_server` handshake rather than the newer interactive deduction-server command protocol. Keep the old handshake for drop-in compatibility, but decide later whether a modern client should speak `ADD`/`RUN`/`QUIT` once `e_server` and `e_deduction_server` executable parity is covered.
- `tcp_msg_wait()` prints every server message, including the expected terminal reply, through `GlobalOut`. Rust preserves this visible echo; a cleaned API should return structured server events separately from presentation.
- The C client accepts reserved ports after a warning and opens/truncates `GlobalOut` before loading input or connecting. Rust keeps both side effects; change them only behind an explicit compatibility-mode decision.
- `FileLoad()` uses `InputOpen()`, which attempts a stat/non-regular-file check separately from file opening. Missing inputs therefore report `Cannot stat file ...`, while directories report `... it is not a regular file`. Keep the split for compatibility; a modernized client should report a single structured input-open failure.
<!-- END MANUAL REVIEW: c_source_docs -->
