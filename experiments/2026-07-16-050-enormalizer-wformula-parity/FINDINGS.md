# enormalizer WFormulaParse parity

## Status

Completed for Bead `E_Rust_Port-j76.1.42`. The formula-target path now matches
C `WFormulaParse` across its complete outer TPTP/TSTP wrapper surface, and the
permanent optimized matrix has grown from 22 to 26 cases. The vendored C source
remained unchanged.

## C dispatch inventory

`WFormulaParse` has three format branches:

- LOP terminates with `OTHER_ERROR` and the exact message
  `LOP currently does not support full FOF!`;
- old TPTP accepts only `input_formula`, names formed by `Name|PosInt`, the
  seven roles `axiom`, `hypothesis`, `negated_conjecture`, `conjecture`,
  `question`, `lemma`, and `unknown`, and no source/useful-info fields; and
- TSTP accepts `fof`, `tff`, `tcf`, and `thf`, names formed by
  `Name|PosInt|SQString`, `type` records under every wrapper kind, all standard
  formula roles, and `watchlist` only under `tcf`.

TSTP body dispatch is also wrapper-sensitive: a leading `$distinct` uses the
distinct parser, `tcf` uses the clause-formula parser, and other records use
the general term-formula parser. Every non-type body is checked for free
variables. An optional source may be a bracketed expression, identifier,
positive integer, or identifier-headed expression; a following useful-info
field must begin with `[`.

The focused Rust matrix now covers all accepted roles, plain/positive-integer/
single-quoted/double-quoted names, all four wrapper kinds, type records under
all four kinds, all source shapes used by the executable matrix, useful info,
TCF-only watchlists, higher-order definition metadata, initial/input formula
properties, and complete scanner consumption. Existing focused tests retain
the body-parser boundaries for `$distinct`, TCF clauses, typed `$let`, free
variables, useful-info rejection, and non-TCF watchlist rejection.

## Parity fixes

Rust previously classified a LOP formula target as a syntax error with a
Rust-specific message. It now returns C's exact message and `OTHER_ERROR`, so
the executable exits 11.

C `WFormulaPrint` cannot produce LOP formula syntax. It emits
`Currently no LOP FOF format, using TPTP` and falls through to the old-TPTP
renderer. `enormalizer` calls it once before and once after normalization, so
the warning appears twice per target. Rust now preserves both warnings and the
TPTP fallback bytes. This duplicate warning is compatibility-visible but has a
dedicated post-compatibility cleanup Bead, `E_Rust_Port-j76.4.1326`.

## Permanent executable matrix

The new `tstp-fo-wrapper-matrix` case combines FOF/TFF/TCF type records, every
non-type role, every accepted name-token family, optional numeric/bracketed/
headed sources, useful-info skipping, and TCF watchlist acceptance. A separate
THF case avoids C's deliberate first-order/higher-order mixing rejection while
pinning THF type, definition, question, quoted-name, numeric-name, and source
behavior. The old-TPTP case now covers every accepted role plus plain,
positive-integer, and double-quoted names.

Two additional cases pin the default LOP-output fallback and the unsupported
LOP-input exit. The optimized native runner asserts exact stable stdout/stderr
for all successful wrapper cases, the two fallback warnings, the exit-11
diagnostic, all earlier error statuses, output-file bytes, and system-error
shapes.

## Reference boundary

The archived built-C report at
`.artifacts/e-compare/20260715-203258-985096-tools/tool-comparison.json` proves
byte equality for help, version, and the original LOP term workload. This
desktop session still has no WSL distribution, cached Windows C executable, or
native POSIX C toolchain, so the four new permanent cases could not be rerun
against C. Their expected behavior comes directly from the reviewed
`ccl_formula_wrapper.c`, `ccl_clauses.c`, and `enormalizer.c` paths and is ready
for the normal differential command when the reference environment returns.

## Validation

- focused `enormalizer` library tests: 41 passed;
- full library suite: 4,157 passed;
- all binary and integration targets passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin enormalizer`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- optimized native `enormalizer` matrix: all 26 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
