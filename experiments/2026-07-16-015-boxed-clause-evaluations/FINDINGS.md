# Boxed clause evaluation cells

## Question

Can Rust match C's nullable `ClauseCell.evaluations` pointer by boxing the
variably sized evaluation descriptor, shrinking unevaluated clauses and the
large whole-clause buffers visible at the post-TSM-fix memory peak?

The clean Rust `Clause` stored `Option<EvalCell>` inline. Because `EvalCell`
contains a `Vec` descriptor, every clause paid for that inactive representation,
including archive copies and the temporary vector used for clause sorting. C
stores `Eval_p evaluations` and allocates it only when heuristic evaluation is
attached.

## Setup and candidate

The saved native Linux baseline is commit `f4f2eebf`, copied before editing to:

```text
.artifacts/experiments/2026-07-16-015-boxed-clause-evaluations/baseline/eprover
```

The temporary candidate changed only `Clause.evaluations` from
`Option<EvalCell>` to `Option<Box<EvalCell>>`, preserving the public borrowed,
mutable, attach, take, and removal APIs. A focused 64-bit layout regression
confirmed that `Clause` shrank from 192 to 152 bytes and that the optional owner
occupied one pointer word. The existing evaluation lifecycle test continued to
exercise attach, mutable access, take, reattach, and removal.

The WSL release candidate was built with:

```bash
cargo build --locked --release --bin eprover
```

Five-run interleaved scaling used:

```bash
bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-015-boxed-clause-evaluations/baseline/eprover \
  target/release/eprover \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-015-boxed-clause-evaluations/raw/scaling-final.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-015-boxed-clause-evaluations/raw/scaling-final.csv
```

Paired Massif runs used the repeated-owner 1,000- and 20,000-owner corpora:

```bash
valgrind --tool=massif --time-unit=B \
  --massif-out-file="$massif_output" \
  "$binary" --cnf --silent --output-file=/dev/null "$problem"
```

## Focused results

The clean scaling rerun showed small process-RSS reductions:

| Shape, owners | Implementation | Wall median (s) | CPU median (s) | RSS median (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | C | 0.320 | 0.080 | 34,704 |
| Repeated, 20,000 | Baseline | 0.220 | 0.180 | 62,300 |
| Repeated, 20,000 | Candidate | 0.220 | 0.180 | 61,956 |
| Unique, 20,000 | C | 0.390 | 0.130 | 51,208 |
| Unique, 20,000 | Baseline | 0.560 | 0.540 | 95,436 |
| Unique, 20,000 | Candidate | 0.530 | 0.520 | 95,256 |

Repeated-owner RSS fell 344 KiB (0.55%), while unique-owner RSS fell 180 KiB
(0.19%). Repeated CPU and wall medians were unchanged; unique medians improved
by 0.02 and 0.03 seconds in this short-run window.

Massif confirmed that the compact clause layout saved useful bytes, while also
showing the cost of one allocation per evaluated clause:

| Owners | Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | Baseline | 3,173,829 | 128,619 | 3,302,448 |
| 1,000 | Candidate | 3,146,931 | 144,733 | 3,291,664 |
| 20,000 | Baseline | 60,549,375 | 2,365,505 | 62,914,880 |
| 20,000 | Candidate | 58,537,197 | 2,685,571 | 61,222,768 |

At 20,000 owners, useful heap fell 2,012,178 bytes (3.32%) and total heap fell
1,692,112 bytes (2.69%), but allocator overhead increased 320,066 bytes
(13.53%). At 1,000 owners, total heap fell only 10,784 bytes (0.33%).

The first scaling file, `raw/scaling.csv`, is invalid: `/usr/bin/time` emitted
an impossible `-39.99`-second wall sample and the analyzer rejected it. The
complete rerun in `raw/scaling-final.csv` has all 150 valid samples and is the
only scaling dataset used for the decision.

## Repository-wide candidate gates

The temporary candidate passed:

- `git diff --check` and `cargo fmt --all -- --check`;
- `cargo check --all-targets --all-features`;
- pedantic Clippy with warnings denied;
- all 4,090 library tests, every binary target, and all three schedule tests;
- locked optimized Linux and Windows `eprover` builds.

The standard 50-case C-vs-Rust report is
`.artifacts/e-compare/20260716-051319-555726/comparison.json`. It retained the
six stable mismatches from the clean baseline and added only normalized proof
text for `LUSK6ext.lop`. Two isolated reruns of the exact candidate both matched
C with zero mismatches:

- `.artifacts/e-compare/20260716-052531-462298/comparison.json`;
- `.artifacts/e-compare/20260716-052609-229344/comparison.json`.

`LUSK6ext.lop` has also appeared intermittently in an earlier accepted full
run, so the extra full-run mismatch is not a stable candidate behavior change.

The standard five-run benchmark report is
`.artifacts/e-compare/20260716-052711-450472-benchmark/benchmark.json`. Aggregate
Rust/C wall time improved from 3.304x to 3.172x, and the known differing
`BOO020-1.p` outcome remained excluded. However, the proof-search memory results
reversed the focused-corpus direction:

| Case | Baseline Rust max RSS (KiB) | Candidate Rust max RSS (KiB) | Change (KiB) |
| --- | ---: | ---: | ---: |
| `LUSK6.lop` | 257,760 | 258,400 | +640 |
| `LUSK6ext.lop` | 503,908 | 506,128 | +2,220 |
| `BOO020-1.p` | 1,880,480 | 1,924,480 | +44,000 |

The 44,000 KiB increase on resource-bound `BOO020-1.p` is 2.34%. It is
consistent with allocator overhead accumulating across many evaluated clauses,
and it moves the existing allocation-abort mismatch farther from C's
resource-limit outcome. The sustained-case increases are smaller but point in
the same direction.

## Conclusion

Reject per-clause boxing of `EvalCell`. The representation matches C's pointer
shape and reduces focused useful heap, but the extra allocation per evaluated
clause worsens operational proof-search RSS, substantially on the resource-bound
case. The aggregate timing improvement does not justify a memory regression that
can cause an earlier allocation failure.

All production and test changes were reverted with `apply_patch`; formatting
restored an empty source diff. A future evaluation-owner optimization should use
pooled or arena-backed storage so clauses retain compact handles without one
system allocation per evaluation cell.
