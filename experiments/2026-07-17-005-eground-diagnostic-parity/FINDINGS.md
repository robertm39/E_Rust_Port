# eground diagnostic parity

## Status

Completed for Bead `E_Rust_Port-j76.2.139`. The permanent optimized `eground`
matrix is now byte-for-byte exact against archived upstream C across all 22
cases, with zero mismatches and zero expected differences. The vendored C tree
remained unchanged.

## Residual differences

The preceding matrix had four diagnostic-only mismatches:

- verbose stdin used Rust's lower-level `Input is coming`/`Closing input`
  wording instead of the scanner's `Opened <stdin>`/`Closing <stdin>` lifecycle;
- malformed and trailing-token stdin diagnostics used source `-` instead of
  `<stdin>`;
- a missing named input reported `Cannot open file` instead of C's earlier
  `Cannot stat file`; and
- the verbose conjecture case omitted two term-bank garbage-collection
  start/reclaimed-count pairs.

## Resolution

The eground scanner adapter now labels stdin `<stdin>`, checks named paths with
the shared `input_open` metadata-first path, and emits successful scanner
`Opened`/`Closing` messages around parsing. Parser failures still return before
the close message, matching C.

Rust was already performing the same threshold/final sweeps inside
`FormulaSetSimplify` and `FormulaSetCNF2`; only their individual counts were
lost behind aggregate result fields. Formula-set simplification/CNF results now
retain recovered counts in execution order while preserving the aggregate
count and sum. Eground renders one C-shaped start/reclaimed pair per actual
sweep before `CNFization done`; it does not run extra diagnostic-only GC.

For the permanent conjecture input, both implementations report two sweeps and
two recovered cells per sweep.

## Exact comparison

Archived C commit: `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

Report:

`.artifacts/e-compare/20260717-021359-566837-tools/`

Result: 22 cases, 0 mismatches, 0 expected differences.

The matrix covers help/version, LOP and TSTP grounding, direct and nested
selected includes, compact non-unit LOP/TPTP/TSTP routes, verbose scanner/CNF
progress, DIMACS split output, malformed/trailing/semantic failures,
unconstrained and constrained give-up behavior, resource options, and missing
input/output paths.

## Validation

- 31 focused `prover::eground` tests pass serially;
- focused formula-set term-GC tests pass;
- 33 comparison-harness tests pass;
- optimized `eground` build passes; and
- the 22-case archived-C/Rust matrix is fully exact;
- all 4,196 library tests and every all-target/all-feature integration target
  pass serially; and
- strict Clippy, formatting, source-doc coverage, Change Later wording, local
  links, and regeneration-preservation gates pass.
