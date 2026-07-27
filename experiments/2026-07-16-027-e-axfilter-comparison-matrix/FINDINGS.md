# Broader `e_axfilter` comparison matrix

## Status

Completed for Bead `E_Rust_Port-j76.1.18` as an evidence-backed comparison
decision with permanent exact-output cases. This host has no installed WSL
distribution, so the new cases could not be executed against a fresh C binary
in this session.

The later current-reference rerun in
[`experiments/2026-07-18-098-axfilter-owner-closure/FINDINGS.md`](../2026-07-18-098-axfilter-owner-closure/FINDINGS.md)
corrects FOL/HO reference routing and confirms all nine cases exactly.

## Existing live reference evidence

The archived WSL report at
`.artifacts/e-compare/20260711-062453-279853-tools/tool-comparison.json` records
exact normalized C/Rust matches for:

- help and version output;
- the built-in filter dump and missing-problem status; and
- a TSTP threshold run, including both `global.out` and the complete generated
  `problem_tiny.p` file.

That live result establishes the executable wrapper, parsing/progress output,
configured-output routing, generated-file collection, and baseline formula
renderer used by the expanded cases.

## Expanded matrix

The permanent tool-comparison cases now add:

- a six-formula GSinE corpus whose goal activates a three-link D-relation chain
  while two unrelated formula owners remain excluded;
- a THF LambdaDef corpus with two definition records plus selected conjecture,
  hypothesis, and question roles and an excluded ordinary axiom;
- an artificial seeded run using `--seed-method=lda --seeds=p`, comparing
  stdout `Name:` ordering, configured `global.out`, and exact
  `problem_SA_P1_24_seed.p`, `problem_SL_P1_24_seed.p`, and
  `problem_SD_P1_24_seed.p` contents; and
- configured-output missing-parent and missing-filter failures in isolated
  working directories.

The matching executable regression pins the seeded symbol code `24`, `SA` then
`SL` then `SD` order, stdout-only names, configured-output-only filter progress,
and each method descriptor in its generated file.

## Platform diagnostics

The C configured-output and scanner-open paths emit a stable diagnostic prefix
followed by the host `strerror` text. The new missing-parent and missing-filter
cases exercise those paths while the shared interop normalizer canonicalizes
the known POSIX and Windows missing-file suffixes.

`OutClose` does not expose the host broken-pipe string here. It replaces any
close/flush failure with the stable message `Output stream to be closed reports
error (probably broken pipe, file system full or quota exceeded)`. Rust's
injected flush-failure regression pins that exact text. The interop normalizer
also already equates POSIX `Broken pipe` with Windows errors 109 and 232 for
other paths that do retain the system suffix.

## Performance decision

Only tests, fixtures, and comparison metadata changed. Production parsing,
selection, rendering, and I/O paths are unchanged, so a benchmark is not
warranted.

## Validation

- focused expanded GSinE regression passed
- focused expanded LambdaDef regression passed
- focused seeded all-methods regression passed
- focused `prover::e_axfilter::tests`: 23 passed
- interop harness tests passed: 26 passed
- Python bytecode compilation for the interop harness and tests
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,109 passed
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
