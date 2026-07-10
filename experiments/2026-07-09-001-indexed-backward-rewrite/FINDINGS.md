# Indexed Backward Rewrite Parity

Date: 2026-07-09

## Question

Does the Rust proof-control path use the configured global backward-rewrite
subterm index in the same way as C, and does that explain the severe
`COL042-8.p` performance mismatch?

## Setup

Baseline C/Rust comparison artifacts are preserved under:

```text
.artifacts/e-compare/20260709-065258-343902/
```

The post-fix full comparison artifacts are preserved under:

```text
.artifacts/e-compare/20260709-224129-729562/
```

The bounded diagnostic command was:

```powershell
target\release\eprover.exe --auto --output-level=1 --processed-clauses-limit=120 --cpu-limit=15 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\TPTP\COL042-8.p --print-statistics
```

The exact theorem attempt was:

```powershell
target\release\eprover.exe --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\TPTP\COL042-8.p
```

The focused regression was run with:

```powershell
cargo test --all-targets --all-features backward_simplification_uses_global_rewrite_index_across_processed_sets
```

## Results

At the 120-processed-clause cutoff:

| Measurement | Rust before | C reference | Rust after |
| --- | ---: | ---: | ---: |
| BW rewrite match attempts | 5,391 | 244 | 244 |
| BW rewrite match successes | 16 | 12 | 12 |
| Processed clauses | 124 | 124 | 124 |
| Generated clauses | 911 | 993 | 911 |
| Total rewrite steps | 496 | 524 | 496 |
| Cached rewrite steps | 112 | 271 | 112 |

The regression passed. It indexes one rewritable clause in each of the four
processed sets and leaves a fifth rewritable clause deliberately unindexed.
The indexed clauses are removed in reverse result-stack order, deleted from all
global indexes, archived, and requeued; the unindexed clause remains live. This
falsifies the possibility that the implementation merely retained the old
plain scans while producing similar counts by chance.

The exact 60-second theorem attempt still returns `ResourceOut`, so indexed
backward-rewrite discovery was a real defect but not the complete
`COL042-8.p` divergence.

The full 50-case comparison still exits nonzero because known port gaps remain,
but mismatches fell from 40 to 34 with no candidate-status regression.
`CSR036+2.p` improved from Rust `ResourceOut` to `Theorem`; `lists.p` and
`sledgehammer.p` retained the immediately preceding port increment's new
`Theorem` results; six synthetic prune/CNF/app-encode modes became normalized
output matches; and `SWW194+1.p` now returns bounded `ResourceOut` rather than
outliving the harness timeout.

## Conclusion

Confirmed: C treats `gindices->bw_rw_index` as authoritative across all four
processed sets. Rust previously ignored it in the main backward-simplification
path and scanned each set. Wiring the index makes the backward-match counters
exactly match C at the diagnostic cutoff.

Remaining limit: generated-clause and rewrite-cache counts still diverge. The
next investigation should find the first selected/generated clause divergence
and audit demodulator selection plus rewrite-link/cache reuse there.
