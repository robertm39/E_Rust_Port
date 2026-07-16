# Learning-corpus parse and output overhead

## Status

Complete for Bead `E_Rust_Port-j76.5.2`. The experiment profiles the flat
10,000-training/10,000-test control isolated by the recursive TSM investigation,
removes avoidable parser and formatter allocation, and gives the usual
single-annotation tree an inline representation. C and Rust output remains
byte-exact in both Flat and Recursive modes.

## Reproduction

Generate the fixed-vocabulary corpus and run alternating native-WSL batches:

```bash
python3 generate-corpus.py artifacts/distinct-10000.tsm
./benchmark-wsl.sh REFERENCE_TSM_CLASSIFY RUST_TSM_CLASSIFY \
    artifacts/distinct-10000.tsm artifacts/flat.csv 9
python3 summarize-results.py artifacts/flat.csv
```

The binaries should be optimized Linux builds. `benchmark-wsl.sh` stages both
on the same native WSL filesystem, alternates execution order, discards program
output, and records wall, user, system, and maximum-RSS measurements. The
profiles below used Valgrind 3.22 Callgrind and Massif on the same corpus scaled
to 1,000 terms per section.

## Profiled causes

The original flat run executed 245,452,974 instructions in Rust versus
106,151,780 in C. Rust spent 82% in parsing. Three non-TSM costs were distinct:

- identifiers were copied into owned strings before every term parse, then
  known symbols were split and allocated again before the signature lookup;
- every parsed term normally has one annotation, but `AnnotationTree` was a
  `std::collections::BTreeMap`, so 2,000 singleton annotations retained
  1,440,000 bytes of mostly unused B-tree leaf capacity in the 1,000-term
  Massif run;
- classification accumulated the complete trace in one `String` and allocated
  a second term string per output line before writing stdout once.

The term parsers now borrow the scanner's dynamic-string view. Signature
insertion fast-paths a known symbol with compatible arity and quoted-name lookup
normalizes through string slices. `AnnotationTree` stores zero or one entries
inline and promotes to a sorted B-tree only for multiple annotations, then
compacts again when deletion leaves one entry. Classification renders terms
directly into a small `BufWriter` adapter while retaining the established C
write/flush diagnostics.

Callgrind fell to 198,709,009 instructions after the parser changes and
194,728,025 after the annotation/output changes, a 20.7% reduction from the
original Rust profile. Massif peak useful heap fell from 3,077,491 to 1,664,309
bytes, a 45.9% reduction. The remaining instruction difference is spread over
the shared scanner and term-bank insertion machinery; it no longer corresponds
to a scaling allocation or a TSM-specific regression.

## Final performance

Nine alternating 10,000-term runs produced:

| Mode | Implementation | Median wall | Median CPU | Median max RSS |
| --- | --- | ---: | ---: | ---: |
| Flat | C | 0.76 s | 0.14 s | 11,840 KiB |
| Flat | Rust | 0.21 s | 0.21 s | 15,152 KiB |
| Recursive | C | 0.81 s | 0.21 s | 21,600 KiB |
| Recursive | Rust | 0.31 s | 0.32 s | 30,352 KiB |

Against the pre-change flat baseline, Rust median wall/CPU/RSS changed from
0.30 s/0.31 s/29,712 KiB to 0.21 s/0.21 s/15,152 KiB: reductions of 30%, 32%,
and 49%. Rust/C flat ratios are `0.276x` wall, `1.500x` CPU, and `1.280x` RSS.
The whole-process CPU ratio still includes Rust's checked scanner and shared
term representation, but the requested workload is 3.6 times faster by
observed wall time and remains within 3.3 MiB of C's peak RSS.

## Compatibility and decision

The final Flat output compares byte-for-byte at SHA-256
`210e0fdac0836775b8a8bd936508f3f20218414b6d3e13e9f40cc463c04f7820`.
The final Recursive output remains byte-for-byte at SHA-256
`f2cc8a910e7d8460b89fe493171d002b466bbd3bed5689dc60c33682ebaa1222`.

Accept the remaining CPU ratio as comparable for this workload: it is bounded,
does not grow through annotation storage or output buffering, and is outweighed
by substantially better measured wall time while memory is close to the C
process. Full unit, documentation, pedantic Clippy, release-build, and 14-case
learning-tool differential gates provide the final compatibility check.

## Validation

The retained candidate passed:

- all 4,178 library tests and every binary/integration target;
- `cargo fmt --check`, `git diff --check`, and all-target/all-feature Clippy
  with warnings and `clippy::pedantic` denied;
- locked Windows and Linux release builds for `eprover` and `tsm_classify`,
  plus a refreshed Windows build of every support binary;
- all C-source documentation coverage, Change Later wording, local-link, and
  manual-regeneration-preservation checks;
- all 14 learning-tool cases with zero mismatches in
  `.artifacts/e-compare/20260716-184931-171120-tools`;
- the 50-case main-prover matrix with no new parser/signature mismatch. Its four
  results outside strict equality are the existing proof-search/resource cases
  `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and the synthetic CPU-limit case,
  recorded in `.artifacts/e-compare/20260716-185320-094090`.
