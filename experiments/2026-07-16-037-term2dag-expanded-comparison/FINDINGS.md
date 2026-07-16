# term2dag expanded comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.29` with an expanded differential matrix,
an exact native regression, and an evidence-backed release-build parser decision.
The vendored C source remained unchanged.

## C surfaces

`SIMPLE_APPS/term2dag.c` deliberately leaves the process-global `problemType`
uninitialized. `TBTermParseReal` accepts a sort annotation on a variable and
calls `TypeBankParseType`. That parser selects its first-order grammar only for
`PROBLEM_FO`; its other branch asserts `PROBLEM_HO` and then parses a curried
higher-order type. The normal upstream build defines `NDEBUG` in
`Makefile.vars`, so the assertion is removed and `PROBLEM_NOT_INIT` follows the
higher-order branch. Rust previously reported `Problem type is not initialized`
instead, making every sorted-variable term fail before DAG construction.

The current-problem Rust type-parser entry now maps only `NotInitialized` to the
higher-order parser mode. Explicit callers of `TypeBank::parse_type` still get a
diagnostic if they explicitly pass `NotInitialized`. This matches the upstream
release executable without spreading an uninitialized global into the typed
Rust API.

## Expanded corpus

`TOOL_FUNCTIONAL_CASES["term2dag"]` now adds
`shared-typed-boundary`, which covers:

- repeated `f(a,a)` and `g(f(a,a))` nodes and repeated top-level terms;
- an `$i`-sorted variable;
- an arrow-sorted variable `F:$i > $i`, producing the curried signature type
  `($i > $i) > $i > $i`;
- a `$o`-sorted variable followed by the same Boolean argument without an
  annotation, reaching the C term-formula coercion through `$eq(Y,$true)`;
- an integer and a distinct object; and
- exact signature order, entry numbers, compact references, expanded terms,
  and internal property integers.

The exact Rust golden includes the representative masks:

- `16390 = TPIsShared | TPIsGround | TPTopPos` for ground top terms;
- `16386 = TPIsShared | TPTopPos` for the variable-containing `h` root;
- `33570818` for the arrow-variable `apply` root, including
  `TPHasEtaExpandableSubterm`;
- `1073758210` for the Boolean-argument `q` root, including
  `TPHasBoolSubterm`; and
- `1610629120` for the generated equality node, including
  `TPHasEqNeqSym | TPHasBoolSubterm`.

These integers follow the C enum values in `cte_termtypes.h` and the same
property propagation in `tb_termtop_insert`. The matrix keeps them byte-strict;
there is no property normalization.

## Platform error boundary

The matrix also adds `missing-input` in an isolated working directory. Native
Windows exits 6 and emits the stable two-line prefix followed by:

```text
term2dag: The system cannot find the file specified. (os error 2)
```

The differential harness canonicalizes only the already-established POSIX and
Windows file-not-found suffixes. Program name, path, prefix, punctuation,
stdout/stderr channel, and exit status remain strict.

## Reference availability and decision

The archived Linux comparison already establishes exact help and basic
binary-function output. This host currently has no installed WSL distribution,
C compiler, surviving standalone C `term2dag` binary, or compatible build
environment, so the expanded corpus cannot be rerun against C in this session.
The source-level release-build control flow and property-mask arithmetic are
deterministic, the complete Rust output is pinned, and the cases will run
against C automatically when the reference environment is restored. This is
the evidence-backed compatibility decision permitted by the migrated work
item's acceptance criteria.

## Performance decision

The parser change affects only type annotations encountered while the global
problem type is uninitialized. Untyped term parsing and DAG construction keep
the same paths, so a performance benchmark is not warranted.

## Validation

- `tools/e-interop/test_e_interop.py`: 27 passed
- focused exact `term2dag` typed/shared regression: 1 passed
- native missing-input probe: exit 6 with the expected Windows suffix
- `cargo test --locked --lib --quiet`: 4,124 passed
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
