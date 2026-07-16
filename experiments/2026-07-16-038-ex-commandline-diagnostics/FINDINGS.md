# ex_commandline diagnostic comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.30` with expanded differential cases,
exact parser regressions, and restored C `SysError` behavior for numeric range
failures. The vendored C source remained unchanged.

## C surfaces

`SIMPLE_APPS/ex_commandline.c` exposes the shared CLIB command-line parser with
one required integer option and one optional floating-point option. Stable
option-shape failures use `Error(...)` and emit one diagnostic line. In
`CLStateGetIntArg` and `CLStateGetFloatArg`, malformed text with `errno == 0`
also uses that one-line path, but `strtol`/`strtod` overflow or underflow saves
`ERANGE` and calls `SysError(...)`. The latter emits the stable first line and a
second `perror` line whose suffix comes from the active C runtime.

Rust previously rejected range failures but rendered them as ordinary malformed
numbers. It now distinguishes `Invalid` from `Range`, obtains saved-errno text
through a narrow safe wrapper over C `strerror`, and includes the second
program-prefixed line. The same safe CRT helper now backs the existing
`TmpErrno` renderer instead of interpreting a C errno as a Win32 error code.

## Platform evidence

Both `msvcrt.dll` and `ucrtbase.dll` report errno 34 as:

```text
Result too large
```

The common glibc spelling is `Numerical result out of range`. The comparison
harness maps only those complete line suffixes to `<C ERROR: RANGE>`. The stable
option description, offending value, program prefix, two-line structure,
stderr channel, and usage exit status 5 remain strict.

Native Windows now emits, for example:

```text
ex_commandline: -i or --int_example expects integer instead of '9223372036854775808'
ex_commandline: Result too large
```

and the analogous two lines for `--float_example=1e9999`.

## Expanded matrix

`TOOL_FUNCTIONAL_CASES["ex_commandline"]` now includes:

- the existing successful mixed-option workload;
- `unknown-long-option`;
- `missing-required-argument`;
- `invalid-integer`, which proves ordinary malformed text remains one-line;
- `integer-range`; and
- `float-range`.

The archived reference setup already covers help and the successful option
workload. This host currently has no installed WSL distribution, compiler, or
surviving standalone C executable, so the new cases cannot be rerun against C
in this session. Their program-authored text follows direct branches in
`cio_commandline.c`; the cases will run automatically when the reference
environment is restored.

## Performance decision

Only invalid CLI inputs enter the new classification and CRT message path.
Successful option parsing is unchanged, so a benchmark is not warranted.

## Validation

- `tools/e-interop/test_e_interop.py`: 27 passed
- focused `simple_apps::ex_commandline::tests`: 8 passed
- focused `inout::commandline::tests`: 13 passed
- focused `basics::error::tests`: 9 passed
- native integer and float range probes: exact two-line text, exit 5
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,126 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed
- `cargo test --locked --test e_stratpar --quiet`: 1 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo build --locked --release --bin ex_commandline`
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`

Two parallel validation attempts hit unrelated global-environment/timing flakes;
both failed tests passed immediately in isolation. The serialized complete run
is green, and follow-up Bead `E_Rust_Port-9wi` tracks test stabilization.
