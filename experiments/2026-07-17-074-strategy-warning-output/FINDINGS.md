# Strategy Missing-field Warning Output

## Status

Completed for Bead `E_Rust_Port-j76.2.64`. Rust now emits strategy-file
missing-field warnings from the executable at the same point and with the same
bytes as C. The vendored C checkout remained unchanged.

## Question

Does `eprover --parse-strategy` expose the warnings collected by the Rust
`HeuristicParmsParseInto`/`OrderParmsParseInto` ports, and are those warnings
ordered like C when a later selected-strategy lookup fails?

## Original gap

C's `strategy_io()` calls `HeuristicParmsParseInto(..., true)`. Missing-field
branches call `Warning()` immediately, before `GetHeuristicWithName()` performs
an optional selected-strategy lookup. Rust's parsers already returned the same
missing fields and warning diagnostics, but `apply_strategy_io_to_params()`
discarded that report. Both normal proof search and `--print-strategy` were
therefore silent.

## Implementation

The executable strategy-I/O path now separates parsing from selected-strategy
application. It writes the parser report after the strategy file has been
validated and before the later selection step, preserving C's ordering. The
pure proof-control construction helper remains available without an output
owner for library callers.

The warning diagnostics also preserve C's unusual blank-line surface. Each
missing-field call passes `"Config misses %s\n"` to `Warning()`, and
`Warning()` appends another newline after formatting the message. Rust retains
the message newline in both the ordering and HCB parser reports, then the common
warning renderer adds the second newline.

## Direct executable comparison

[`compare_strategy_warnings.py`](compare_strategy_warnings.py) prints Rust's
default higher-order strategy, removes exactly `ordertype` and `db_w`, and runs
the resulting sparse in-order override against the isolated unchanged
`ENABLE_LFHO` C executable at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release executable.
The generated fixture SHA-256 is
`C85ACAF3C3A65D2D4461FAF8BC5A57C93D7D4EAD6302ADA70E3F378F6FAAE54B`.

| Case | Exit | stdout SHA-256 | stderr SHA-256 | Result |
| --- | ---: | --- | --- | --- |
| Normal proof search | `1` | `E0CB9978F236C6D96CE8B69AED7BCD4641FA3850EAA1907D9686C465F24ED221` | `7C9DEBA6A22B7FDCB13209F235AA4389A7766FC2657535D02B546CEE6C679277` | exact |
| Later missing selection | `11` | `47967A8133A91A3D663FEAC76E24DEAEB083969AC9006A4C8CD6577A4BA887D4` | `123606B5340648C335C21F11364AE4DDA9289445DA0B6EDC2969F64E068C9898` | exact |

The second case's exact standard error is the two missing-field warnings,
including C's blank lines, followed by
`eprover: Error: Configuration name Missing not found.` The full retained
outputs are in [`reference.json`](reference.json), whose SHA-256 is
`B84E4F5E487FD63AF965DB5622EA7BB47FE8B82FB2472C6A4D37FEB1C5253120`.

## Permanent regressions

Two executable-driver unit tests pin the warning output through both owners:

- normal proof search reaches a satisfiable result and emits both exact
  warnings; and
- print-strategy handling emits both warnings before returning the later
  missing-selection diagnostic.

The existing ordering and HCB parser-report tests continue to pin sparse-field
preservation and complete missing-field collection.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-074-strategy-warning-output\compare_strategy_warnings.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --rust-exe target\release\eprover.exe `
  --output-dir target\strategy-warning-output `
  --output target\strategy-warning-reference.json `
  --expected experiments\2026-07-17-074-strategy-warning-output\reference.json

cargo test --locked --all-features `
  run_print_strategy_emits_parse_warnings_before_later_selection_errors
cargo test --locked --all-features `
  proof_search_emits_strategy_file_missing_order_warnings
```

## Compatibility decision

Missing strategy fields remain non-fatal sparse overrides, but executable
callers now expose every warning exactly where C does. This migrated item is
implemented behavior rather than remaining port work.
