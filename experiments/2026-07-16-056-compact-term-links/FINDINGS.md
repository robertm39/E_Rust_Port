# Compact term link storage

## Question

Can Rust reduce the live term-node memory gap by matching C's inline nullable
`TermCell` pointer fields without unsafe Rust, per-term auxiliary allocations,
or changes to term identity and sharing?

C stores `binding`, rewrite replacement, type, and left/right term-tree links
as nullable pointers in `TERMS/cte_termtypes.h`. Rust represented each field as
its own `RefCell<Option<_>>`, so every term node paid for five separate borrow
flags in addition to the five pointer-sized option values.

## Candidate

The candidate groups the five pointer values into one private `TermLinks`
record behind a single `RefCell`. Existing `Term` accessors retain the same
owned-clone and mutation APIs, `Rc` remains the term identity, argument storage
is unchanged, and no unsafe code or new allocation is introduced.

On 64-bit targets, the permanent layout/lifecycle regression pins:

- `TermLinks`: 40 bytes;
- `RefCell<TermLinks>`: 48 bytes;
- `TermCell`: 152 bytes, down from the former 184-byte layout.

The same test exercises binding, rewrite-replacement, type, left-link, and
right-link set/get/take behavior through the public owner APIs.

## Exact baseline construction

The Windows baseline executable was built and copied before the source edit.
For Linux, a first exploratory run used the cached binary from experiment 015,
but that binary predates later production commits and is not used for causal
claims. Its `raw/scaling.csv` and old-baseline Massif profile remain only as an
audit record.

The decision baseline is pristine commit `bf999928`, exported with `git
archive` before the uncommitted candidate change. The two compile-time vendored
inputs, `HEURISTICS/schedule.vars` and `PROVER/e_options.h`, were copied from
the unchanged `eprover/` checkout into the ignored archive tree. The exact
baseline and candidate were built with the same WSL Rust toolchain into
separate ext4 target directories.

The upstream C checkout was never modified.

## Focused formula-owner scaling

The existing interleaved harness ran C, exact baseline Rust, and candidate Rust
five times at 100, 1,000, 5,000, 10,000, and 20,000 formula owners for both the
repeated-term and unique-atom corpora:

```bash
bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" "$baseline_binary" "$candidate_binary" \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-056-compact-term-links/raw/scaling-exact.csv
```

At 20,000 owners, native Linux medians were:

| Shape | Implementation | Wall (s) | CPU (s) | RSS (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated | C | 0.34 | 0.08 | 34,240 |
| Repeated | Exact baseline | 0.16 | 0.17 | 61,984 |
| Repeated | Candidate | 0.18 | 0.18 | 61,988 |
| Unique atom | C | 0.45 | 0.13 | 50,704 |
| Unique atom | Exact baseline | 0.44 | 0.47 | 95,284 |
| Unique atom | Candidate | 0.47 | 0.50 | 93,244 |

The repeated corpus interns a small term population and remains flat within
4 KiB. The unique corpus allocates about 60,000 live term nodes and falls 2,040
KiB (2.14%). Its short-run CPU/wall medians move by 0.03 seconds, within the
measurement resolution and without a superlinear trend.

The reproducible Windows sampler in `benchmark-windows.ps1` alternates exact
baseline and candidate processes and samples working set every two
milliseconds. Nine-run medians independently show unique-atom peak working set
falling 91,496 to 89,968 KiB (-1,528 KiB, -1.67%) and wall time falling 0.597
to 0.570 seconds. Repeated-owner working set is flat at 62,532 versus 62,544
KiB.

## Massif live heap

Paired native-Linux Massif profiles on the 20,000-owner unique corpus used the
exact baseline and candidate binaries:

| Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| --- | ---: | ---: | ---: |
| Exact baseline | 88,047,882 | 8,086,630 | 96,134,512 |
| Candidate | 86,127,746 | 8,086,750 | 94,214,496 |

Useful live heap falls 1,920,136 bytes while allocator bookkeeping rises only
120 bytes. Total live heap falls 1,920,016 bytes (2.00%). The useful-byte delta
corresponds to approximately 60,004 compacted term nodes at 32 bytes each,
independently tying the measured reduction to the representation change.

## Exact proof-search controls

`benchmark-proof-wsl.sh` alternated exact baseline and candidate binaries with
the standard benchmark options. Five-run results on the two sustained matched
cases were:

| Case | Implementation | Wall median (s) | CPU median (s) | Max RSS (KiB) |
| --- | --- | ---: | ---: | ---: |
| `LUSK6.lop` | Exact baseline | 2.89 | 2.72 | 257,600 |
| `LUSK6.lop` | Candidate | 2.83 | 2.74 | 241,236 |
| `LUSK6ext.lop` | Exact baseline | 6.66 | 6.34 | 503,708 |
| `LUSK6ext.lop` | Candidate | 6.46 | 6.19 | 467,728 |

Peak RSS falls 16,364 KiB (6.35%) on `LUSK6` and 35,980 KiB (7.14%) on
`LUSK6ext`. Wall medians improve 2.1% and 3.0%; the small `LUSK6` CPU movement
is +0.7%, while `LUSK6ext` CPU improves 2.4%. Both outcomes and statuses remain
unchanged.

One exact-baseline/candidate `BOO020-1.p` resource control preserves the known
allocation-abort outcome in both binaries. Candidate max RSS is 1,888,884 KiB
versus 1,879,996 KiB (+8,888 KiB, +0.47%) and CPU is 37.10 versus 34.76
seconds. This layout-sensitive search trajectory is a small adverse boundary
movement, but it neither changes the already-known result nor approaches the
rejected boxed-evaluation candidate's 44,000 KiB/2.34% regression. The
successful sustained cases and exact heap accounting remain strongly positive.

## Repository-wide compatibility and performance

The 50-case C/Rust comparison report is
`.artifacts/e-compare/20260716-155752-854814/comparison.json`. It has four
established mismatches: `BOO020-1.p`, `SWV851-1.p`, `sledgehammer.p`, and the
synthetic CPU-limit case. This is a strict subset of the six-mismatch accepted
baseline; no new case or comparison field differs.

The standard candidate-vs-C five-run benchmark is
`.artifacts/e-compare/20260716-160952-510104-benchmark/benchmark.json`. Its
aggregate Rust/C wall ratio is 3.202x, with only the known `BOO020-1.p` outcome
excluded. This absolute report is not used as the candidate delta because its
older comparison report predates intervening source changes; the exact
baseline controls above isolate the patch.

## Conclusion

Retain compact `TermLinks`. It mirrors C's inline pointer ownership with one
safe Rust mutation boundary, removes exactly 32 bytes from every 64-bit term
node, reduces unique-owner live heap by 2.00%, and reduces sustained
proof-search RSS by 6-7% without changing successful behavior or slowing wall
time. The small adverse movement at the pre-existing `BOO020` allocation-abort
boundary is documented and does not outweigh the repeatable live-memory gains.

This is another partial formula-owner and proof-search memory improvement, not
completion of the task. Focused unique-owner Rust RSS remains about 1.84x C,
and the standard Rust/C wall ratio remains 3.202x, so further ownership and hot
path work is still required.

## Validation

The retained candidate passed:

- all 4,169 library tests and every binary target;
- all four `eprover_schedule` tests plus the `e_stratpar`, executable-inventory,
  and real LTB variant-worker integrations;
- `cargo check --locked --all-targets --all-features`;
- canonical all-target Clippy with warnings denied;
- Rust formatting and `git diff --check`;
- locked release builds for native Windows and Linux `eprover`;
- all 32 Python interop-tool tests;
- C-source documentation coverage, Change Later wording, local links, and
  manual-section regeneration preservation.

The standards-prescribed `-D clippy::pedantic` pass found existing warnings in
unrelated parser, process-control, server, OS-wrapper, and test code; none refer
to this candidate. Restoring that repository-wide gate is tracked as
`E_Rust_Port-j76.6` rather than hiding the findings with a blanket allowance,
and the pedantic pass is not presented as green for this slice.
