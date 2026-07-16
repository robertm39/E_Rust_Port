# epcllemma expanded comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.34` with a relative-limit arithmetic
parity fix, expanded formula/large/error comparison cases, exact unusual-float
regressions, and evidence-backed build/platform decisions. The vendored C
source remained unchanged.

## Single-precision relative limit

C computes the default absolute lemma limit with:

```c
max_lemmas = PCLProtStepNo(prot) * max_lemmas_rel + 0.99;
```

`max_lemmas_rel` is a `float`, so the protocol step count is converted to
single precision and the multiplication is rounded as a `float` before the
double `0.99` is added. Rust previously converted both inputs to `f64` before
multiplication. That changed a real boundary: for 1,010 steps at the default
relative value, the C intermediate is `1.0099999904632568` and the final
integer limit is `1`, while the promoted Rust calculation produced `2`.

Rust now performs the multiplication in `f32` before the addition. The focused
regression and permanent `large-relative-limit` matrix case parse 1,010 UPCL2
steps, require the exact `% Selecting at most 1 lemmas` line, and preserve the
sequential C selector's separately documented off-by-one behavior by printing
two selected lemmas when the minimum quality is zero.

The conversion of an out-of-range floating result to C `long` is undefined by
C and is not a portable comparison surface. Ordinary representable values,
including the large boundary above, are exact.

## Formula-valued selected lemmas

A two-step formula protocol forces its first FOF step to quality zero, selects
it with the zero absolute threshold, and stops through the C-compatible
`max_lemmas == 0` sequential path. Four permanent cases pin the resulting
lemma-only output:

- PCL: the step is marked with external type `lemma`;
- TPTP: `input_formula(1,lemma,p(a))`;
- TSTP: `fof(1,lemma,p(a),unknown()).`; and
- LOP: the bare `p(a)` formula.

The second formula remains unselected, so these cases distinguish selected
lemma output from the already-covered level-3 full-protocol path. The LOP case
also continues to exercise C's option fallthrough back to the iterative
algorithm.

## Unusual floating-point spelling

`CLStateGetFloatArg` parses through `strtod`, the executable stores the result
in `float`, and the status line prints it with `%f`. Rust already preserved the
double-to-single conversion but Rust's formatter spells NaN as `NaN`, unlike
the lowercase glibc C spelling used by the reference build. The executable now
has a narrow C-fixed formatter:

- finite values retain exactly six fractional digits, including `-0.000000`;
- infinities are `inf` and `-inf`; and
- NaNs are `nan` or `-nan` according to the stored sign bit.

Permanent empty-protocol cases pin `nan`, positive and negative infinity, and
negative zero without introducing selector comparisons involving NaN.

## Shell syntax and file diagnostics

`epcllemma.c` never enables `SupportShellPCL`. The new shell-step case pins its
syntax rejection in the executable matrix. Isolated missing-input and missing
output-parent cases cover both scanner and `OpenGlobalOut` error paths. Their
program/path prefixes remain byte-for-byte strict; only the already-established
complete POSIX/Windows not-found suffixes are canonicalized.

The lower-level executable tests retain the exact C `OutClose` message for a
flush failure. A real POSIX C writer can instead terminate through `SIGPIPE`
before `OutClose`, while native Windows reports a write error. The comparison
harness intentionally keeps each capture pipe's reader open, so manufacturing
that condition would require a host-specific outer-process test. Rust keeps
deterministic checked writes and final flushes rather than emulating
signal-dependent termination.

## `STACK_SIZE` build boundary

The only C `STACK_SIZE` behavior is a preprocessor-guarded
`INCREASE_STACK_SIZE` call before `InitIO`. The macro expands to POSIX
`getrlimit(RLIMIT_STACK)` followed by an attempt to set the soft limit to the
hard limit, with warning fragments if either operation fails. No source build
configuration defines `STACK_SIZE`, so the normal archived C reference does
not execute this branch.

Rust therefore matches the default executable and does not mutate a process
resource limit. Its protocol storage is heap-owned, and proof sizes are cached
while serialized steps are visited; the 1,010-step case directly exercises the
expanded protocol surface without a stack-setting dependency. If an explicit
stack-raised Unix distribution target is ever required, it should expose that
policy through its launcher/build configuration rather than changing the
portable executable unconditionally.

## Reference availability and performance

This sandbox has no visible WSL distribution, compiler, or archived C tools,
so the expanded cases cannot run against C in this session. They remain in the
permanent matrix and will exercise the archived C executable in the normal
user-context reference environment. The changed runtime operations are one
single-precision multiplication and a tiny status formatter; the 1,010-step
case completes within the focused test suite, so a separate performance
benchmark is not warranted.

## Validation

- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,133 passed
- `cargo test --locked --bins --quiet -- --test-threads=1`: all binary targets passed
- `eprover_schedule`, `e_stratpar`, and `executable_inventory` integration
  suites: 4, 1, and 1 passed
- `tools/e-interop/test_e_interop.py`: 30 passed
- focused `prover::epcllemma::tests`: 23 passed
- `cargo check --locked --all-targets`: passed
- `cargo clippy --locked --all-targets -- -D warnings`: passed
- `cargo build --locked --release --bin epcllemma`: passed
- `cargo fmt --all -- --check`: passed
- C-source documentation coverage: 492 source files across 266 unit docs
- Change Later wording and local-link checks: 269 Markdown files each
- documentation regeneration: preserved manual sections in 268 files
- native 1,010-step, TSTP-formula, NaN, and shell-rejection probes: expected
  output and status
