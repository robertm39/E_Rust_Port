# `--error-on-empty` selected-owner reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.92`. Rust now uses the same selected
proof-state owner domain, aggregate multi-file decision point, error code, and
output framing as the reference `eprover`.

## Question

Does `--error-on-empty` test the number of clauses produced by lowering, the
number of parsed records, or the number of selected non-watchlist clause and
formula owners? Does it test each file or the complete input set?

## Reference behavior

The unchanged vendored C source is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. `parse_spec` parses every command-
line input into one `ProofState`, processes `$distinct`, and only then tests
`ProofStateAxNo`. The macro sums ordinary axiom clauses and formula axioms; it
does not count the watchlist.

Consequently:

- TFF type declarations and a top-level `$true` formula count as nonempty even
  when later CNF output contains no clauses.
- An input containing only comments or only a watchlist owner is empty.
- An empty file does not fail when another input file contributes an ordinary
  selected owner.

With the reference `PRINT_SOMEERRORS_STDOUT` build, rejection has exit status
11 (`OTHER_ERROR`) and these exact streams:

```text
stdout: % Error: Input file contains no clauses or formulas\n% SZS status InputError\n
stderr: eprover: Input file contains no clauses or formulas\n
```

## Live comparison

[`compare_cases.py`](compare_cases.py) runs the Windows release binary and the
cached WSL C reference in syntax-only and silent CNF-only modes. The six cases
are comment-only, type-only, `$true`-only, watchlist-only, an ordinary clause,
and a comment-only file followed by the ordinary-clause file.

All 12 comparisons matched exactly after CRLF normalization: acceptance, exit
status, stdout, and stderr. The compact retained outcome is in
[`results-summary.json`](results-summary.json); the harness prints the complete
per-case transcripts when rerun.

## Rust reconciliation

The executable parser already tracked `input_owner_seen` separately from the
lowered clause count, which correctly represented type and `$true` owners while
excluding watchlist owners. The remaining incompatibilities were at the
executable boundary:

- per-file checks were removed so the decision is made once after every input;
- a distinct internal empty-input error maps to `OTHER_ERROR` and the reference
  message; and
- `run_config_with_stderr` catches only that error long enough to emit the
  reference error comment and `InputError` status through configured
  `GlobalOut`, then flushes before returning it to the binary wrapper.

Other parser and semantic diagnostics retain their existing error codes and
messages. The added branch is error-only and has no successful-run allocation,
parsing, or proof-search cost, so no performance benchmark is warranted.

## Reproduction

From the repository root after a release build:

```powershell
& 'C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' `
  experiments\2026-07-17-045-error-on-empty-owner-count\compare_cases.py `
  --rust-exe target\release\eprover.exe `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --distro Ubuntu-24.04
```

## Validation

- 12 live C/Rust executable comparisons: exact
- 9 focused Rust `error_on_empty` regressions: passed
- permanent regressions cover syntax-only and CNF type/`$true` owners, exact
  rejection output and configured-output routing, watchlist exclusion,
  app-encode include side output, and aggregate multi-file behavior
- full serial suite: 4,252 library tests plus all binary/integration targets
- strict all-target/all-feature pedantic Clippy: passed
- formatting and all four C-source documentation integrity gates: passed
