# Optional fingerprint-index statistics

## Status

Completed for Bead `E_Rust_Port-j76.2.45`. The existing Rust
`print-index-stats` feature is an exact executable replacement for C's optional
`PRINT_INDEX_STATS` block on the represented first-order index configurations.
No production defect was found. One permanent regression now pins C's zeroed
distribution formatting when paramodulation indexes are disabled.

## Reference build

The C reference was built from the isolated commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` source copy at:

```text
/home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-fpstats-20260718a
```

Its normal optimized first-order flags were retained, with only
`-DPRINT_INDEX_STATS` added. The repository's vendored `eprover/` checkout was
not modified. Rust was built with:

```powershell
cargo build --locked --release --bin eprover --features print-index-stats
```

## Executable comparison

[`compare_index_stats.py`](compare_index_stats.py) runs both feature-enabled
executables over [`index-stats.p`](index-stats.p) with deterministic FIFO
selection, no preprocessing or generation, and three processed clauses. The
three cases cover:

- FP1's compact one-sample paths;
- FP7's deeper paths, with 22 DOT trie nodes; and
- FP1 rewrite/paramod-from indexes with disabled paramod-into and negative-atom
  indexes, exercising C's null-index distribution format.

Raw pointer values are diagnostic process addresses, so the comparison renames
them by first occurrence while preserving every repeated node/payload
relationship. After that normalization, all 3/3 cases are exact for:

- the four global-index distribution lines and their field widths;
- node, leaf, mean, and standard-deviation values;
- DOT graph framing, node order, path labels, and structural edges;
- flattened subterm payload records and leaf-to-payload edges;
- SZS status, exit code, and empty stderr.

The retained report is 1,367 bytes with SHA-256
`61F584D43072B933454DB7B6CA2DDC296B1F56EC355E0175593C06B80EEF4FD2`.
The FP1 and disabled-index blocks each contain 33 lines; the deep FP7 block
contains 69.

## Owner reconciliation

C prints the optional block at the end of `eprover` proof statistics. It emits
backward-rewrite and paramod-from distributions, the `pm_from_index` DOT graph,
then paramod-into and negative-atom distributions. `FPIndexDistribDataPrint`
renders absent indexes as zero nodes/leaves with zero mean and deviation.

Rust's feature gate reaches the proof-state-owned live `GlobalIndices` at the
same final-statistics position. Its generic FP-tree renderer preserves payload
paths, distribution aggregation, DOT nodes before edges before payloads, and
the flattened `SubtermTreePrintDot` record branch. The new feature-gated unit
test fixes the exact two-line disabled-index tail in place.

[`audit_index_stats.py`](audit_index_stats.py) pins 18/18 C-owner, Rust feature,
writer-order, renderer, and regression-test contracts.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-092-fingerprint-index-stats\audit_index_stats.py `
  --repo . `
  --output target\fingerprint-index-stats-audit.json `
  --expected experiments\2026-07-18-092-fingerprint-index-stats\audit-reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-092-fingerprint-index-stats\compare_index_stats.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-fpstats-20260718a/PROVER/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\fingerprint-index-stats.json `
  --expected experiments\2026-07-18-092-fingerprint-index-stats\comparison-reference.json
```

## Validation

- optional executable comparison: 3/3 exact;
- source/test audit: 18/18 contracts passed;
- both global-index statistics regressions passed with the feature enabled; and
- full suite, strict lint/format gates, documentation gates, feature-enabled
  optimized build, and vendored-C cleanliness are recorded in the completing
  commit.
