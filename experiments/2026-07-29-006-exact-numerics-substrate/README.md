# Exact integer/rational substrate study

Bead: `E_Rust_Port-9jt.5.1`

This experiment compares four replaceable exact-rational backends without
adding any of them to Umlaut:

- `num-rational` 0.4.2 with `num-bigint` 0.4.8;
- `dashu-ratio` 0.5.1 with `dashu-int` 0.5.1;
- `rug` 1.30.0 over its pinned full-GMP FFI dependency; and
- GMP 6.3.0's Mini-GMP plus Mini-MPQ C fallback.

All backends consume the same line-oriented vectors. A Python
`fractions.Fraction` oracle independently computes canonical results for
construction, addition, subtraction, multiplication, division, floor,
ceiling, and comparison. Backend-independent FNV-1a digests make any
normalization or arithmetic disagreement fail closed.
The `--corrupt-oracle` diagnostic deliberately flips one expected digest bit
so this fail-closed path can itself be falsified and retained as evidence.

The workload generator includes rational constants extracted from the tracked
paper-derived `viras_docs/` packet plus deterministic small, medium, and large
random operands. Timed loops exclude parsing and decimal serialization.

The Rust crate is an experiment-only package with its own lockfile. Its
dependencies are not added to the repository root package, and none of the
experiment binaries are shipped.

See [`FINDINGS.md`](FINDINGS.md) for exact source revisions, build commands,
the transitive license matrix, property and performance results, falsification
checks, and retained evidence. The reversible production-facing contract is
specified separately in [`INTERFACE.md`](INTERFACE.md).
