# PDTree Variable-Edge Metadata Cache

## Question

Can the compact PDTree variable-edge arena snapshot each indexed variable's
type UID and standard weight at insertion, avoiding repeated reference-counted
type access and weight-property checks during live-substitution traversal?

## Setup

- Baseline commit: `efd6ef0c` (`Reuse PDTree query root weight`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-010-pdt-variable-metadata-cache/baseline-eprover`.
- Baseline SHA-256:
  `7587e0362ee04d9e5fe7356dbdb621bbe69bfcf4392800e023730bf640de5850`.
- Candidate SHA-256:
  `f36149bca32414b059313ebcd47bb2e5bd3bbefe9489a80d61912f15ef241d46`.
- Primary problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Falsification problems:
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p` and
  `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`.
- Common proof options:
  `--auto --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

Rebuild and rerun from the repository root:

```bash
cargo build --locked --release --bin eprover
bash experiments/2026-07-14-010-pdt-variable-metadata-cache/benchmark.sh
bash experiments/2026-07-14-010-pdt-variable-metadata-cache/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-010-pdt-variable-metadata-cache/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

## Implementation

Each live `PdtVariableChild` now stores the `type_uid` and `weight` already
present in its insertion `PrefixToken::FreeVar`. Live-substitution traversal
uses those immutable edge fields for the type guard and variable-edge weight
adjustment instead of reopening the indexed `Term` on every visit.

The free-list allocator overwrites both fields whenever it reuses an arena
slot. A focused test deletes an individual-typed variable edge, then reuses the
same slot for a Boolean applied-variable edge with a different type UID and
standard weight, proving both cached values refresh.

## Results

Seven alternating LUSK6 pairs measured baseline/candidate medians of
`3.59`/`3.55` user seconds and `3.55`/`3.50` wall seconds, improvements of 1.1%
and 1.4%. Two baseline wall runs were externally delayed, so Callgrind is the
primary acceptance signal. Raw timings are retained in the ignored experiment
artifact directory.

Matched Callgrind runs execute `20,021,308,767` baseline instructions and
`19,889,029,245` candidate instructions. The reduction is `132,279,522`, or
0.66%. `search_next_matching_occurrence_with_subst` falls from
`1,337,404,051` to `1,203,687,953`, a `133,716,098` reduction of almost 10%.
The roughly 1.4 million instruction offset outside the traversal is the cost
of populating the wider edge records.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 70.67 s | 69.58 s | Unsatisfiable |
| GEO288+1 | 62.34 s | 61.28 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated,
`1,062,557` paramodulation, and `1,022,255` rewrite-step counters. GEO288
retains exact `10,215` processed, `128,583` generated, `127,990`
paramodulation, and `34,170` rewrite-step counters. GEO288's
allocation-sensitive non-redundant subcounter differs by four, within the
documented raw-address ordering behavior. The elevated absolute times affect
both binaries and are not used as cross-session evidence.

The full 50-case Windows-Rust/WSL-C differential report is retained at
`.artifacts/e-compare/20260714-205018-173289/`. It reports five established
port gaps: resource/exit behavior for `BOO020-1.p` and `SWV851-1.p`, normalized
proof text for `LUSK6ext.lop` and `sledgehammer.p`, and the synthetic one-second
CPU-limit boundary. `LUSK6.lop`, `GEO288+1.p`, and `HEN011-2.p` all match C in
status and normalized output in this run.

The three-run native Linux benchmark is retained at
`.artifacts/e-compare/20260714-210617-328216-benchmark/`. Its aggregate Rust/C
wall-time ratio is 3.199x, reflecting the existing whole-port performance gap.
LUSK6 measures 2.700 seconds wall and 2.93 seconds CPU for Rust, a 2.387x wall
ratio to C; LUSK6ext measures 6.195 seconds wall and 6.75 seconds CPU, a 2.640x
wall ratio. Both outcomes match C. The known `BOO020-1.p` behavior mismatch is
excluded from the aggregate ratio.

## Falsification Checks

- All 34 focused `clauses::pdtrees` tests pass.
- The full all-target, all-feature suite passes 4,055 library tests and 3
  schedule integration tests.
- Strict all-target, all-feature Clippy passes with warnings and pedantic lints
  denied.
- C-source-doc regeneration, manual-section preservation, Change Later wording,
  and Markdown-link checks pass.
- The free-slot reuse test changes both cached metadata values.
- LUSK6 improves in alternating exact-binary medians and deterministic
  instruction count.
- HEN011 and GEO288 preserve status and principal saturation counters.
- Both scripts pass `bash -n` and use repository-relative paths.

## Conclusion And Limits

The metadata cache is accepted because it removes repeated ownership-heavy
field access from the hot variable traversal, cuts that function's instruction
count by almost 10%, and preserves proof behavior. It adds two 64-bit fields to
each Rust variable-edge record, so later memory profiling should retain the
cache only while the traversal gain outweighs the arena footprint.

C directly rereads `Term_p` fields and does not need the same cache today, but
that relies on indexed shared-term type and weight immutability. A later C term
storage redesign should encode that invariant explicitly and consider edge
snapshots only if metadata access becomes indirect enough to justify the extra
per-edge storage.
