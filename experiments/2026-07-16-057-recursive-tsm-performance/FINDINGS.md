# Recursive TSM performance

## Status

Complete for Bead `E_Rust_Port-j76.5.1`. The experiment isolates fixed process
startup from scaled recursive classification, reproduces the original gap,
profiles both heap and CPU behavior, restores C's shared-substitution ownership,
and removes a deep copy from every symbol-index lookup. Exact output and the
14-case learning-tool differential remain unchanged.

## Workloads

`generate-scaled-corpus.py` provides two scaling modes over the permanent
`recursive-mixed.tsm` fixture:

- `--repeats` duplicates the original annotations. This was useful as a control,
  but it is not a scaling workload: `AnnoSet` coalesces duplicate terms, leaving
  only the original 12 unique nodes and a 27-line result.
- `--distinct-terms` generates unique logarithmic-depth terms from the fixed
  `a`, `b`, `f`, and `g` vocabulary. This exercises recursive TSM growth without
  conflating it with signature growth.

`benchmark-scaled-wsl.sh` stages both executables together on WSL's native
filesystem, alternates implementation order, and records wall, user, system,
and maximum-RSS measurements. Its optional trailing arguments also permit the
flat control run. `summarize-results.py` computes the reported medians and
ratios from the raw CSV files.

## Reproduced gap

Before the fixes, distinct recursive corpora showed a real scaling failure:

| Distinct training + test terms | Implementation | Median wall | Median CPU | Median max RSS |
| --- | --- | ---: | ---: | ---: |
| 1,000 + 1,000 | C | 0.05 s | 0.01 s | 4,800 KiB |
| 1,000 + 1,000 | Rust | 0.09 s | 0.09 s | 19,680 KiB |
| 10,000 + 10,000 | C | 0.70 s | 0.19 s | 21,600 KiB |
| 10,000 + 10,000 | Rust | 1.23 s | 1.34 s | 175,316 KiB |

At 10,000 terms this was `1.757x` by wall time, `7.053x` by sampled CPU,
and `8.116x` by median maximum RSS.

## Profiles and causes

Paired Massif profiles on the 1,000-term corpus recorded C peak heap at
2,142,562 bytes and Rust at 15,112,157 bytes. Rust's dominant allocations were
the `Vec<Tsm>` backing store and repeated deep copies of `PatternSubst`,
including the substitution's `Signature` vectors and maps. The corresponding C
headers explicitly mark `IndexTermCell.subst`, `TSMIndexCell.subst`, and
`TSMAdminCell.subst` as shared pointers.

After restoring shared ownership, Rust 10,000-term median RSS fell from 175,316
to 44,912 KiB, but wall time remained 1.31 seconds. Callgrind then attributed
606,959,525 of 1,176,417,122 instructions (`51.59%`) to `TSMIndex::find`.
`index_symbol_key` cloned the complete substitution on every lookup because
Rust's logically read-only `PatternSubst::symbol_value` API unnecessarily
required `&mut self`. C documents `PatSymbValue` as side-effect free.

The implementation now:

- stores one `Rc<PatternSubst>` across the admin, recursive TSM indexes, and
  index terms, while preserving the public owned constructors and deliberate
  deep-copy behavior elsewhere;
- performs symbol-value and original-symbol reads through immutable array
  access, returning the same zero value for an unbound out-of-capacity symbol
  without growing the backing array;
- tests pointer identity across index construction and every non-empty TSM,
  and tests that read-only symbol lookup does not grow substitution storage.

## Final performance

The final distinct-corpus measurements are:

| Distinct training + test terms | Implementation | Median wall | Median CPU | Median max RSS |
| --- | --- | ---: | ---: | ---: |
| 1,000 + 1,000 | C | 0.06 s | 0.01 s | 4,800 KiB |
| 1,000 + 1,000 | Rust | 0.02 s | 0.02 s | 7,040 KiB |
| 10,000 + 10,000 | C | 0.87 s | 0.21 s | 21,600 KiB |
| 10,000 + 10,000 | Rust | 0.39 s | 0.38 s | 44,912 KiB |

Rust/C wall ratios are `0.333x` at 1,000 terms and `0.448x` at 10,000 terms.
The 10,000-term flat control measured C/Rust wall at 0.72/0.30 seconds and CPU
at 0.14/0.31 seconds. Recursive incremental CPU is therefore 0.07 seconds for
both implementations; the remaining aggregate CPU and memory difference is
already present in parsing, annotation storage, and output formatting rather
than recursive TSM construction.

The permanent 12+12 fixture remains intentionally startup-sensitive. Across
11 alternating batches of 200 processes, its final medians are:

| Implementation | Wall / 200 | CPU / 200 | Peak RSS |
| --- | ---: | ---: | ---: |
| C | 0.91 s | 0.36 s | 3,200 KiB |
| Rust | 1.05 s | 0.36 s | 3,200 KiB |

CPU is at parity. The residual `1.154x` wall ratio is about 0.7 ms per process
and does not scale with TSM work. Stripping a temporary Rust binary from
1,617,064 to 1,167,800 bytes did not improve the relative startup result, so
release symbol-table size is not the cause.

## Compatibility

The final 10,000-term C and Rust outputs contain 35,907 lines and 1,713,660
bytes. They compare byte-for-byte with SHA-256
`f2cc8a910e7d8460b89fe493171d002b466bbd3bed5689dc60c33682ebaa1222`.

The optimized Windows candidate also passes all 14 archived-C learning-tool
cases in `.artifacts/e-compare/20260716-180458-358149-tools` with zero
mismatches.

## Decision

Accept and close `E_Rust_Port-j76.5.1`. The relevant recursive scaling cost is
now equal by incremental CPU and substantially faster by wall time; the only
ratio above `1.10x` is the explicitly isolated fixed process-startup wall cost,
for which the Bead permits an evidence-backed compatibility decision. The
remaining flat parsing/output CPU and RSS difference is outside the recursive
TSM regression and must not be attributed to this data structure. It is tracked
separately as Bead `E_Rust_Port-j76.5.2`.
