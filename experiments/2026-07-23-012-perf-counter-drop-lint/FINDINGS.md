# Feature-gated performance-counter drop lint

## Status

Accepted in Experiment 250 for Bead `E_Rust_Port-j76.5.3`.

## Trigger

Rust 1.94 strict default-feature Clippy reports `clippy::drop_non_drop` for the
explicit saturation-timer boundary in `run_proof_search`:

```text
error: call to `std::mem::drop` with a value that does not implement `Drop`
```

`PerfCounterGuard` implements `Drop` when `instrument-perf-ctr` is enabled, but
is deliberately a zero-sized no-op without that feature. The explicit drop must
remain before proof-search filtering and output so instrumented `SatTimer`
statistics cover saturation only.

## Rejected helper shape

A consuming `PerfCounterGuard::finish(self)` method passed default and
all-feature Clippy and preserved feature-enabled RAII semantics. It was rejected
after release disassembly showed that the default build enlarged the
`run_proof_search` stack frame from `0x89c8` to `0x89d8` bytes. Adding
`#[inline]` did not remove that 16-byte frame increase.

The accepted Experiment 245 and helper-candidate `.text` hashes were:

- Accepted: `c1b328eedd9d6bbd03ac465917a2eff7de8e71de38aa9aeb9857da2ad50e7e10`
- Helper: `b641aeea8ebcc8a24c20f8584ac17f11784bef1d498d56f3d40f8d4aa1d7b38f`

Both executables remained 8,654,336 bytes with a `0x4ef168`-byte `.text`
section, so section size alone would not have caught the changed stack layout.

## Accepted fix

The original `drop(sat_timer)` remains unchanged. A `cfg_attr` applies a
reasoned `clippy::drop_non_drop` expectation only when instrumentation is
disabled. With instrumentation enabled, the guard still has its original
`Drop` implementation and no lint expectation is emitted.

The final default release `.text` hash is exactly the accepted hash:

`c1b328eedd9d6bbd03ac465917a2eff7de8e71de38aa9aeb9857da2ad50e7e10`

Therefore native timing and interop matrices are not repeated: the default
Windows executable code is byte-for-byte identical, while the instrumented
configuration retains the original statement and drop behavior.

## Artifact handling note

GNU `objcopy --dump-section` rewrote PE container metadata in place while
leaving `.text` unchanged. The ignored accepted binary was regenerated from the
accepted source at the original workspace path. Its `.text` remains exact;
whole-PE hashes vary across link times, so the final comparison used a
non-mutating byte-range hash.

## Validation

- `cargo clippy --locked --lib -- -D warnings -D clippy::pedantic`
- `cargo clippy --locked --all-targets --all-features -- -D warnings -D clippy::pedantic`
- `$env:CARGO_BUILD_JOBS='1'; cargo test --locked --all-targets --all-features --quiet`
  - 4,388 library tests passed
  - all binary and integration targets passed
- `$env:CARGO_BUILD_JOBS='1'; cargo build --locked --release --bin eprover --all-features`
- `cargo fmt --all -- --check`
- `check_markdown_links.py`: 269 Markdown files checked
- accepted/final default release `.text`: byte-for-byte equal
- vendored `eprover/` status: clean

The serial build-job setting avoids Windows paging-file exhaustion while rustc
maps the large all-feature library into multiple binary test processes.
