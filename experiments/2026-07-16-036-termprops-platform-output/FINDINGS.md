# termprops platform output

## Status

Completed for Bead `E_Rust_Port-j76.1.28` as an expanded comparison matrix and
an evidence-backed platform-formatting decision. The vendored C source remained
unchanged.

## C surfaces

`PROVER/termprops.c:147-151` prints both averages with `%f` after dividing by
`(float)count`. Empty input therefore computes zero divided by zero twice and
delegates the spelling to the target C library. Input opening goes through
`CreateScanner`, whose stable diagnostic prefix is followed by the host
`strerror(errno)` result.

The existing archived Linux comparison covered help and a non-empty stdin term
stream. This host currently has no compiler, installed WSL distribution, or
remaining standalone C binary, so a fresh C run for the new edge cases is not
available.

## Target evidence

The native release Rust executable produces:

```text
% Terms: 0  ASize: nan MSize: 0, ADepth: nan MDepth: 0
```

For an isolated missing input it exits 6 and writes:

```text
termprops: Cannot open file missing-termprops-reference-input for reading
termprops: The system cannot find the file specified. (os error 2)
```

The Windows legacy `msvcrt.dll` formatting probe prints positive and negative
quiet NaNs as `1.#QNAN0` and `-1.#IND00`; its `strerror(2)` is `No such file or
directory`. Existing glibc-family reference evidence elsewhere in this project
uses `-nan` for the same zero-denominator shape. These results demonstrate that
raw non-finite and host-error spelling is not one portable byte sequence even
when the program-authored text is identical.

## Expanded comparison matrix

`TOOL_FUNCTIONAL_CASES["termprops"]` now includes:

- `stdin-basic`, the existing non-empty reference case;
- `empty-input`, which reaches both NaN average fields; and
- `missing-input`, run in an isolated working directory so no ambient file can
  accidentally satisfy the open.

Normalization remains narrow:

- known POSIX and Windows file-not-found suffixes become
  `<OS ERROR: NOT FOUND>`;
- only a NaN token immediately following `ASize:` or `ADepth:` becomes
  `<NAN>`; and
- exit status, stdout/stderr channel, stable prefix, path, punctuation, all
  finite values, and unrelated NaN text remain strict.

The accepted NaN token set covers C99 `nan`/`nan(payload)` forms and legacy
Microsoft `1.#IND`, `1.#QNAN`, and `1.#SNAN` forms with their sign and trailing
digits. Unit tests compare glibc-shaped, Rust-shaped, and legacy-Microsoft
summaries and prove that an unrelated `-nan` line is untouched.

## Compatibility decision

Rust keeps its native `nan` output rather than emulating one foreign CRT. A
drop-in build should match stable program-authored behavior on its host; the
differential harness must isolate the two runtime-supplied fields when comparing
different operating systems. The new cases will execute against the C binary
automatically when the reference environment is restored.

## Performance decision

Only comparison cases, normalization, tests, and documentation changed.
Production term parsing and rendering are unchanged, so a benchmark is not
warranted.

## Validation

- `tools/e-interop/test_e_interop.py`: 27 passed
- focused `prover::termprops::tests`: 12 passed
- native release empty-input and isolated missing-input probes
- `cargo test --locked --lib --quiet`: 4,123 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed
- `cargo test --locked --test e_stratpar --quiet`: 1 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
