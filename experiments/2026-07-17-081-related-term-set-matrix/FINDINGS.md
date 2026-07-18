# Related-term-set consumer matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.56`. All four `RelatedTermSet` modes run
through the production owners of all six conjecture-term consumers. The
vendored C checkout remained unchanged.

## Question

Do `ConjectureRelativeTermWeight`, `ConjectureTermPrefixWeight`,
`ConjectureTermTfIdfWeight`, `ConjectureLevDistanceWeight`,
`ConjectureTreeDistanceWeight`, and `ConjectureStrucDistanceWeight` preserve
their related-term mode through executable heuristic selection?

## Matrix

[`compare_related_term_sets.py`](compare_related_term_sets.py) runs the six
families with each of these four C enum values:

- `0`: conjecture terms;
- `1`: conjecture subterms;
- `2`: conjecture subterms plus top generalizations; and
- `3`: conjecture subterms plus all generalizations.

Each case processes one clause from [`problem.p`](problem.p) and compares the
complete C/Rust stdout, stderr, and exit code. The fixture contains two axioms
with different term shapes and a related but nonidentical conjecture, so the
weight functions make an observable selection rather than tying trivially.

The defined-semantics C comparison is byte-exact in all 24 cases. The retained
[`reference.json`](reference.json) has SHA-256
`56A4D834AA014964F393E98FB5204CC3E62F87B68D744301A195B84FB71D9253`.

## Stock-C undefined boundary

The unchanged C executable is exact in 20 of 24 cases. Its only differences
are the four TF-IDF modes, all of which select the flat axiom while Rust selects
the negated conjecture. [`stock-observed.json`](stock-observed.json) retains
that raw report and has SHA-256
`8D9BEF4CB92543B0BD402C3A18783CCA9CBBB3AA9A7A63B4595998FEE58BD753`.

Source and GDB inspection found an upstream undefined-memory boundary rather
than a Rust collection or owner bug. `ConjectureTermTfIdfWeightInit` parses a
`tf_fact` argument but never assigns it to `data->tf_fact`; the evaluator later
reads that field. Two independent isolated debug builds both read the same
allocator residue for this workload:

```text
tf_fact = 9.7327217552419279e+241
bits    = 0x722d3138302d3731
```

That residue suppresses TF-IDF contributions whose conjecture frequency is
zero. The stock C clause scores are approximately `0.434918`, `0.434918`, and
`0.729218`; Rust's deterministic use of the parsed factor produces
`1.434918`, `1.434918`, and `0.729218`. The permanent proof-control regression
pins the defined Rust scores and evaluates all four related-term modes through
the active HCB.

The GDB command files [`inspect_eqn_ext.gdb`](inspect_eqn_ext.gdb) and
[`inspect_tfidf_terms.gdb`](inspect_tfidf_terms.gdb) retain the clause- and
term-level probes. [`inspect_tfidf.gdb`](inspect_tfidf.gdb) retains the initial
HCB-level probe.

## Compatibility decision

Rust keeps the parsed `tf_fact` and the source-level formula. Reproducing the
stock executable would require importing allocator-history-dependent undefined
behavior, so that raw value is not a safe compatibility target.

For defined-source validation, [`c_tfidf_factor_init.patch`](c_tfidf_factor_init.patch)
adds only the missing assignment to an isolated copy of C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The patch has SHA-256
`EB12A8F49A29C8F3BD4330BBB06F5C3C430C2F0C304EE94E9A2F7172D69C843E`.
The resulting isolated executable has SHA-256
`9CB082B5F8C9AE332A070920F380F2632E23D43C810297DEB94CC99358B7CCBF` and
is byte-exact with Rust in all 24 cases. No file under `eprover/` was modified.

## Reproduction

Build Rust and run the strict defined-semantics comparison:

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-081-related-term-set-matrix\compare_related_term_sets.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-tfidf-defined-20260718a/PROVER/eprover-ho `
  --c-variant tfidf_factor_initialized `
  --rust-exe target\release\eprover.exe `
  --output target\related-term-set-reference.json `
  --expected experiments\2026-07-17-081-related-term-set-matrix\reference.json
```

Run the stock-C command with `--c-variant stock` and the cached FOL executable
to reproduce the intentional 20/24 raw report. That command exits nonzero
after writing the mismatch evidence.
