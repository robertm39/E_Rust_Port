# ESession subprocess-readiness compatibility audit

## Status

Completed for Bead `E_Rust_Port-j76.1.15` as an evidence-backed compatibility
decision. Full subprocess result handling is implemented in the process-control
owner for real callers, but the legacy E session loop deliberately retains C's
descriptor-registration/no-I/O asymmetry.

## Question

Does the C session implementation have unported subprocess I/O semantics, or is
the apparent gap an unimplemented upstream placeholder that a drop-in Rust port
must leave inert?

## C source evidence

`ESessionInitFDSet` always registers the session socket for reading, registers
it for writing when the channel has queued output, and delegates a non-null
`running` set to `EPCtrlSetFDSet`. That helper traverses every process control,
adds its output-pipe descriptor to the read set, and returns the largest
descriptor.

`ESessionDoIO` then begins with:

```c
if(session->running)
{
   /* Do I/O for the controlled processes */
}
```

There is no statement in that block. The function proceeds directly to socket
read/write handling. It does not find a ready control, read a result line,
update a prover result, remove a completed process, or forward process output
to the client.

The missing operations are not hidden in the readiness helper. The separate
`EPCtrlSetGetResult` function owns `select`, `EPCtrlGetResult`, result-state
handling, no-proof deletion, and `GlobalOut` reporting, but neither
`cco_esession.c` nor `cco_eserver.c` calls it. `ESessionProcessCmds` only prints
received client messages and does not create a running process set.

## Rust decision

Rust already represents the complete C boundary:

- `SessionProcessSet::init_read_fd_set` lets `ESession::init_fd_set` add running
  process descriptors and include their maximum in the returned descriptor;
- `EPCtrlSet` implements that trait using its descriptor-indexed process map;
- `ESession::do_io` handles only the channel descriptor; and
- the fully ported `EPCtrlSet` result APIs remain available to batch and
  scheduler owners that actually call them.

Adding result consumption to `ESession::do_io` would not complete a C feature.
It would invent process lifecycle, output-routing, and client-protocol behavior
that the reference server never defines. The Rust function now documents the
empty C branch explicitly.

## Regression

The focused session regression installs a synthetic running-process set with a
descriptor larger than the socket. It proves that initialization registers
both descriptors and returns the process maximum. It then marks only the
process descriptor ready and calls `do_io`, asserting that:

- the session remains active;
- the running set remains attached;
- no client message is read; and
- no `wait` response is queued.

This pins both halves of the asymmetric contract and will catch either loss of
descriptor registration or accidental invention of session subprocess I/O.

## Performance decision

The production behavior is unchanged: one comment makes the compatibility
boundary explicit, and the process-ready no-op performs no work. No performance
benchmark is warranted.

## Validation

- focused `control::esession::tests` passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,100 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
