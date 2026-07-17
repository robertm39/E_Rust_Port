# Deduction-server RUN framing reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.93`. A real loopback Rust regression now
pins the complete TCP-message bytes for the intended C `RUN` sequence. A live
WSL build also exposed and preserved evidence for a default-C PID-prefix bug;
Rust deliberately keeps the usable server behavior instead of reproducing that
bug.

## Question

Does the Rust TCP executable path read `RUN` blocks and emit the same framed
bytes as C, including the fork-child progress/proof messages followed by the
parent success status?

## Reference build

The vendored C commit was
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. It had no built deduction server,
so `git archive` created a source copy under
`C:\tmp\e-ref-j76-2-93-20260717`. The populated PicoSAT submodule was copied
into that temporary tree, Windows line endings were normalized only there, and
WSL Ubuntu 24.04 built `PROVER/e_deduction_server` plus `PROVER/eprover`. The
vendored `eprover/` tree remained unchanged.

[`capture_c_run_frames.py`](capture_c_run_frames.py) reserves a loopback port,
starts the temporary C server, sends one TCP-string `RUN` command plus its
formula and exact `GO\n` terminator frames, decodes every response including
the four-byte network-order length, sends `QUIT`, and terminates the listener.

## Live stock-C result

The stock default build and its matching real `eprover` do not reach the
documented finish frame. `eprover.c` prints its PID with
`fprintf(..., COMCHAR " Pid: ...")`; default `COMCHAR` is the printf-escaped
string `"%%"`, so the emitted line starts with one percent sign. In contrast,
`ECtrlCreateGeneric` passes the same macro directly to `strstr`, where no
formatting occurs, and therefore searches for two percent signs. The child
aborts with `Cannot get eprover PID`; after `wait`, the parent still sends the
success status.

The exact result in [`c_stock_run_frames.json`](c_stock_run_frames.json) is:

1. `\n% Processing started for reference_job\n`
2. `200 ok : success\n`

The `--unix-comments` build avoids the mismatch because `COMCHAR` is `"#"` in
both contexts. The defect belongs to the existing post-compatibility
process-control review Bead `E_Rust_Port-j76.4.662`.

## Intended C path

[`fake_prover.c`](fake_prover.c) is a deterministic process-control fixture. It
emits the doubled-percent PID prefix actually searched by the default C build,
then a theorem status and proof line. This lets the unchanged C server execute
the path after PID validation. The exact transcript in
[`c_intended_run_frames.json`](c_intended_run_frames.json) contains four TCP
messages:

1. start marker, total wire length 44 bytes;
2. complete runner output, total wire length 70 bytes in the captured sample;
3. finish marker, total wire length 46 bytes; and
4. success status, total wire length 21 bytes.

This agrees with the source call order: the `run_command` child sends start,
`BatchProcessProblem` sends runner output, the child sends finish and exits,
then the waiting parent returns the success string for the command loop to
send.

## Rust decision and regression

Rust keeps the functioning, intended protocol. Reproducing the default PID
macro bug would make `RUN` unusable and violate the port requirement to support
the original feature. The safe process controller already recognizes the real
single-percent E PID line and owns the spawned child directly.

`serve_tcp_client_with` is a narrow test seam used by the production client
wrapper and the regression. The new real-`TcpStream` test sends the same three
input frames, verifies the block delivered to the `RUN` handler, and compares
all four received byte vectors with the captured C headers and payloads. It
therefore covers `TCPReadTextBlock`, command dispatch, frame-offset slicing and
TCP-message packing on one loopback path rather than only testing those layers
independently.

## Performance decision

Production still creates one term bank/control state and enters the same
generic command loop once per accepted client. The helper extraction adds no
buffering, allocation or socket operation; it only exposes the existing runner
closure boundary for the loopback test. No benchmark is warranted.

## Validation

- live stock-C WSL transcript captured
- live intended-path C WSL transcript captured
- focused Rust byte-level loopback regression passed
- 14 focused deduction-server tests passed
- 50 focused interactive-mode tests passed
- 8 focused simple-I/O tests passed
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings -D clippy::pedantic`
- `cargo test --locked --all-targets --all-features --quiet -- --test-threads=1`: 4,249 library tests plus all target tests passed
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
