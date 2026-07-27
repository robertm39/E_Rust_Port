# Reporting, strategy, and limit matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.88`. The focused executable surface is
11/11 byte-exact against the pinned C reference. The vendored C source remained
unchanged.

## Question

Do the represented reporting, saturated-state filtering, strategy I/O, and
search-limit options still have an observable C/Rust compatibility gap, and
which residual concerns belong to narrower durable tasks?

## Method

[`compare_surfaces.py`](compare_surfaces.py) runs the Windows release binary and
the isolated WSL C reference from commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. It compares exit status, stdout,
and stderr without normalization. The script shell-quotes C arguments before
passing them through WSL so reserved strategy selectors such as
`>all-names<` are not interpreted as shell redirections.

The fixtures cover:

- current, named, selected, parsed, and all-name strategy printing;
- processed-clause and saturated-state limits;
- descriptor-selected saturated output and its no-op filter bridge;
- proof-found saturated output and maintained statistics; and
- answer-limit termination on the vendored answer smoke test.

## Findings

The first comparison found that every successful C `--print-strategy` path
starts with its unconditional preprocessing-configuration comment, while Rust
started directly with the strategy payload. C emits that comment before
`strategy_io`, so it also precedes parse/select handling. Rust now writes the
same stdout side-channel line before applying a parsed or selected strategy.
Permanent regressions pin the current, named, all-name, and selection-error
ordering.

After the change, every case is byte-exact:

| Case | Exit | Stdout bytes | Exact |
| --- | ---: | ---: | :---: |
| current strategy | 0 | 4,569 | yes |
| all strategy names | 0 | 12,903 | yes |
| named strategy | 0 | 4,877 | yes |
| selected strategy | 0 | 4,877 | yes |
| parsed strategy | 0 | 4,877 | yes |
| processed-clause limit | 9 | 202 | yes |
| saturated limit | 9 | 425 | yes |
| saturated no-op filter | 9 | 425 | yes |
| proof-found saturated output | 0 | 269 | yes |
| statistics | 1 | 2,624 | yes |
| answer limit | 0 | 151 | yes |

The compact hashes and complete mismatch payloads, if a future run regresses,
are retained in [`results-summary.json`](results-summary.json).

## Scope decision

This broad migrated item is an umbrella, not the remaining owner for every
reporting or strategy concern. Exact strategy-I/O timing after input parsing and
preprocessing remains under `E_Rust_Port-j76.3.98`; generated strategy and HCB
integration under `.2.76`/`.2.77`; schedules and fork-state accounting under
`.2.35`, `.3.79`, and `.3.80`; resource limits under `.2.31`; proof-object and
final executable reporting under `.2.30`, `.2.32`, `.2.33`, and `.2.47`; and
the documented C descriptor/default quirks under `.3.70`, `.3.72`, and `.3.73`.
Closing this umbrella does not close those narrower tasks.

## Validation

- focused strategy-print regressions: passed;
- release `eprover` build: passed; and
- focused C/Rust matrix: 11/11 exact;
- default-parallel all-target/all-feature suite: 4,257 library tests plus every
  binary and integration target passed;
- strict all-target/all-feature pedantic Clippy: passed; and
- formatting and all four C-source documentation integrity gates: passed.
