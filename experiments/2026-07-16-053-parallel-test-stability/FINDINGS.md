# Parallel test stability

## Status

Accepted for Bead `E_Rust_Port-9wi`. This experiment isolates and removes two
parallel-only assumptions exposed while validating the `ex_commandline`
diagnostic slice. No production behavior changed, and the vendored C source
remained unchanged.

## Scanner environment reproduction

`Scanner::from_file_following_includes` intentionally reads process-global
`TPTP` when it expands `include(...)`. The include-splicing regression supplied
absolute fixture paths but assumed that `TPTP` was unset. In parallel,
`inout::initio` tests temporarily set `TPTP=Problems`, causing the scanner test
to attempt a path such as `Problems/C:/.../child.ax`.

The failure is deterministic with:

```powershell
$env:TPTP = 'Problems'
cargo test --locked --lib inout::scanner::tests::include_key_splices_included_files_and_resumes_parent_stream -- --exact
```

The test now takes the repository-wide global-state lock, saves the caller's
`TPTP`, unsets it for the fixture, and restores it on scope exit. The same
hostile-environment command therefore passes without changing the production
include-prefix contract or the developer's ambient environment.

## Hard-limit output ordering

Hard CPU-limit finalization deliberately writes the resource-status banner
directly to the configured global output before the pending stdio-compatible
buffer is flushed. This reproduces the C signal path. A zero-second deadline
can, however, be observed at different points under host load. If ordinary
output crossed the 8 KiB compatibility buffer boundary first, some earlier
text is already visible and the resource banner is not the first byte of the
captured stream.

The end-to-end regression now asserts the stable contract: CPU-limit exit
status, exact stderr diagnostic, one exact resource-status banner, the
preprocessing-class output, and absence of the soft/user-limit diagnostic. A
separate deterministic unit regression writes a pending normal fragment, then
a direct fragment, and proves the direct fragment precedes the pending buffer
when the output object is dropped. This retains exact coverage of the direct
signal-output behavior without coupling the executable test to deadline
observation timing. The test also resets the process-global deadline before
evaluating assertions so an assertion failure cannot contaminate later tests.

## Reproduction and stress runner

`run_stress.ps1` repeats the hostile scanner case, the deterministic buffering
case, the hard-limit executable case, and the complete default-parallel library
suite. It preserves and restores the caller's process-level `TPTP` value.

## Validation

- hostile `TPTP=Problems` scanner regression: passed;
- deterministic direct-write/buffer-order regression: passed;
- isolated hard CPU-limit regression: passed;
- `run_stress.ps1 -Iterations 3`: all focused checks passed in every iteration;
- default-parallel library suite: 4,165 passed in each of three consecutive
  runs;
- all binary targets passed;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `git diff --check`: passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
