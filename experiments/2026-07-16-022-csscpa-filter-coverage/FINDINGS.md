# Expanded CSSCPA filter compatibility coverage

## Status

Completed for Bead `E_Rust_Port-j76.1.13`. The permanent support-tool matrix
now includes a 72-clause-command stateful corpus and a missing-input diagnostic,
Rust has exact regressions for the resulting state and host-error composition,
and the larger corpus exposed two compatibility defects that are now fixed.

## Question

Can the remaining `CSSCPA_filter` gap be resolved without pretending that a
Linux `strerror(errno)` suffix and a Windows `std::io::Error` suffix are
byte-identical, and does a larger clause population reveal behavior not covered
by the previously exact single-clause and trace-heavy C/Rust cases?

## Expanded corpus

`CSSCPA_LARGE_STATEFUL_CORPUS` contains 40 forced clauses and 32 checked
clauses. It populates positive-unit, negative-unit, and non-unit buckets, then
exercises:

- 12 unit-subsumed candidates;
- four tautologies;
- eight improvements that replace non-units with units;
- four opposite-sign unit contradictions; and
- four weighty rejections.

It also crosses every `from` source value, switches output level, requests state
twice, and accepts the historical buffering plea. The final state contains 44
clauses and 44 literals. A Rust wrapper regression pins every outcome count,
the final state, the 44 rendered clauses, and representative retained/removed
predicates.

## Defects found

### Runtime signature-code collision

C's first-order build defines `TermIsPhonyApp(term)` as `false`, so the CSSCPA
signature can allocate ordinary symbols through numeric code 17 even though
that number is reserved in the higher-order build. Rust supports both problem
types in one runtime and always recognizes code 17 as the phony-application
code. The former CSSCPA allocator reserved only `$true` and `$false`; its 15th
ordinary user symbol therefore reached a debug assertion and would be
misclassified by release term traversals.

CSSCPA now inserts the fixed internal signature block before parsing user
symbols, as the other runtime-polymorphic Rust front ends do. Visible symbol
names, clause identifiers, weights, and clause-set order are unchanged.

### Subsumed-clause removal order

C traverses matching clause sets in insertion order, pushes every match onto a
`PStack`, and removes them by popping the stack. Rust collected the same vector
but formerly removed it forward, reversing compatibility-visible multi-clause
removal trace lines. Rust now drains that vector in reverse. A regression spans
the positive-unit and non-unit buckets and pins the exact last-non-unit,
first-non-unit, positive-unit trace order.

## Platform diagnostic decision

C's `SysError` suffix is supplied by host `strerror`; Rust's suffix is supplied
by `std::io::Error`. Raw cross-OS byte equality is therefore neither possible
nor a stable compatibility contract. The stable surface remains strict:
program prefix, file path, `for reading` wording, newline placement, stderr
channel, and exit status 6. `normalize_output` canonicalizes only the known
Linux and Windows not-found suffixes to `<OS ERROR: NOT FOUND>`.

New unit tests prove that both output-file and scanner conversion helpers retain
an arbitrary host suffix byte-for-byte before that comparison-layer
normalization. The isolated `CSSCPA_filter/missing-input` case makes the full
process behavior a permanent differential test.

## Reference evidence

The archived report at
`.artifacts/e-compare/20260711-045946-440709-tools/` establishes exact normalized
C/Rust output for help, version, silent acceptance, and the configured
non-silent state/check trace. The expanded corpus composes the same parsers,
state lines, clause traces, and final bucket printers. Direct source audit adds
the two previously missing ordering facts: C `ClauseSetInsert` appends, and C
`CSSCPAProcessClause` removes the reverse of its collected `PStack` order.

No WSL distribution or locally executable C tool cache is installed in this
environment, so the two new matrix cases could not receive a fresh live C run.
This item is closed as an evidence-backed compatibility decision: every stable
output primitive was already exact, the larger composition is pinned on Rust,
the C-only ordering behavior is now represented directly, and the unavoidable
OS suffix is isolated rather than weakening any other comparison field. The
matrix will run the expanded cases automatically when the reference
environment is restored.

## Optimized candidate check

The reproducible `check_windows_candidate.py` run used the release
`CSSCPA_filter.exe`. Its raw report is stored under
`.artifacts/experiments/2026-07-16-022-csscpa-filter-coverage/candidate.json`.

| Case | Exit | Stdout bytes | Stderr bytes | Result |
| --- | ---: | ---: | ---: | --- |
| `large-stateful-corpus` | 0 | 5,784 | 0 | all outcome/final-state checks passed |
| `missing-input` | 6 | 0 | 143 | exact stable prefix and normalized `ENOENT` passed |

The large output SHA-256 is
`22c9712e0c643d2f842341af7a4da054c7ed458988033f2a16732e9110cc8fe3`.
Twenty fresh-process runs measured a 9.393 ms median, 8.816 ms minimum, and
12.821 ms maximum wall time. This is a startup-dominated smoke measurement, not
a C/Rust throughput claim. The correctness fixes retain linear collection and
removal; reserving the small fixed signature block adds constant startup work.

## Validation

- 28 focused CSSCPA library tests passed
- 25 support-tool harness tests passed
- optimized Windows candidate checks passed
- all edited Python files passed bytecode compilation
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,096 passed
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
