# Experiment 290: In-place clause sign partition

## Status

Accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can `Clause::alloc` reuse its already-owned literal vector while performing
the stable positive-before-negative partition required by upstream E?

## Baseline

- Accepted source: commit `13e6949a`, with production code unchanged from
  accepted Experiment 286.
- Accepted exact LUSK6 Callgrind: 8,828,399,104 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Accepted Rust/C ratio: 1.680204.
- The accepted profile calls `Clause::alloc` 120,802 times and attributes
  exactly 120,802 `RawVec::grow_one` calls and 33,712,465 inclusive
  instructions to its fresh sign-partition vectors.

## Candidate

Consume the `EqnList` into its existing vector and stably partition it in
place. This preserves the original relative order of positive literals and of
negative literals, matching C `ClauseAlloc`'s two append chains. It retains the
input allocation instead of allocating positive and negative vectors and
appending them.

The retained implementation recursively partitions each half and rotates the
two middle runs, bounding movement to O(n log n) without unsafe code. The
regressions cover both a four-literal interleaving and 64 alternating literals,
checking the full stable result order.

## Validation and measurement

### Inline partition

The first candidate keeps the stable rotation loop directly in
`Clause::alloc`. It passes the focused clause tests, strict all-feature
library pedantic Clippy, formatting, and the exact proof-status gate.

It retires 8,963,593,909 instructions, a regression of 135,194,805 or
1.531363%. Direct partition work is not the cause: slice rotation accounts
for only 2,778,400 instructions, and the former 33,712,465-instruction
`RawVec` growth edge disappears.

Instead, the crate-wide optimizer stops inlining `term_deref_always`, creating
a new 276,328,019-instruction standalone owner. Substitution backtracking also
gains 46,187,338 standalone instructions. This reproduces a known sensitive
code-generation boundary rather than a cost inherent to the partition.

### Out-of-line partition

Variant B moves only the linear scan and stable rotations behind
`#[inline(never)]`, keeping `Clause::alloc` small without changing the
partition or adding an annotation to the dominant dereference path. It retires
8,965,056,549 instructions, a regression of 136,657,445 or 1.547930%, and
retains the same outlined dereference boundary.

### Combined forced wrapper boundary

Variant C retains the out-of-line partition and forces the single-caller
`term_deref_always` wrapper inline. Experiment 221 rejected that annotation
when its parent already inlined the wrapper; this candidate instead has a
measured 276,328,019-instruction standalone owner. This bounded combination
recovers that actual boundary and retires 8,806,605,432 instructions:

- delta: -21,793,672 instructions;
- improvement: 0.246859%;
- Rust/C ratio: 1.676056.

It removes exactly 120,804 global-allocation calls. The out-of-line partition
costs 8,093,668 instructions inclusive versus the removed
33,712,465-instruction growth edge. `Substitution::norm_term` rises 6,073,187
instructions, matching the known cost of pinning the wrapper boundary.

Three accepted-parent and eight candidate native proof-output runs are
identical. Two 64-pair native blocks improve wall means 0.869651% and
0.709065%, and CPU means 0.444691% and 0.893688%. Combined wall and CPU means
improve 0.789446% and 0.668648%; paired means improve 0.684788% and 0.584349%.
The candidate wins 85 wall and 70 CPU pairs, with 20 CPU ties. Combined last
halves remain positive by 0.107969% wall and 0.262320% CPU.

This candidate is not retained as implemented because rotating every later
positive literal across the accumulated negative prefix has quadratic
worst-case movement on long alternating clauses.

### Divide-and-conquer stable partition

Variant D recursively partitions each half and rotates the middle negative and
positive runs into place. It preserves the same stable result and allocation
reuse without unsafe code, while bounding movement to O(n log n) instead of
the preceding quadratic algorithm.

It preserves the exact proof and improves further to 8,800,386,737
instructions:

- delta: -28,012,367 instructions;
- improvement: 0.317298%;
- Rust/C ratio: 1.674873.

Three accepted-parent and eight final-candidate native proof-output runs are
identical. The default-feature Windows candidate is 8,931,840 bytes, 32,768
bytes smaller than the 8,964,608-byte parent.

After a four-pair warmup, two independent 64-pair blocks both improve native
wall and CPU means:

| Window | Wall mean | CPU mean | Paired wall mean | Paired CPU mean |
| --- | ---: | ---: | ---: | ---: |
| Block 1 | -0.882660% | -0.260659% | -0.800721% | -0.186124% |
| Block 2 | -0.770107% | -0.649592% | -0.681582% | -0.564801% |
| Combined | -0.826306% | -0.455433% | -0.741151% | -0.375462% |

Negative values are improvements. Across all 128 pairs, the final candidate
wins 78 wall and 58 CPU pairs, with 26 CPU ties. Combined last halves improve
wall and CPU means 0.922802% and 1.077266%, and paired means 0.833048% and
0.993721%. Combined final quarters improve mean wall and CPU by 2.052638% and
1.777120%.

The final profile retains 4,266,671 global-allocation calls versus 4,387,475
in the parent, a reduction of 120,804 or 2.753383%. The bounded partition
costs 2,778,446 instructions inclusive. Global allocator edge cost falls from
341,343,060 to 337,070,702 instructions.

The final measured rows are `native-divide-lusk.csv` and
`native-divide-lusk-2.csv`. The earlier `native-lusk.csv` files retain the
rejected quadratic variant's otherwise-positive timing evidence.

## Compatibility and validation

- Three accepted-parent and eight final-candidate native proof-output runs all
  exit zero with identical 378-character `Unsatisfiable` output and SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- Maintained report `.artifacts/e-compare/20260724-123042-247185` completes all
  50 cases with zero mismatches and only the declared `sledgehammer`
  normalized-output difference. It includes exact LUSK6, HEN011, BOO020, and
  SWV851 proof/resource outcomes.
- The complete serial all-target/all-feature suite passes 4,394 library tests
  plus every integration and binary target.
- Strict all-target/all-feature pedantic Clippy and the locked all-feature
  release build pass.
- Formatting, `git diff --check`, all four C-source documentation gates, and
  vendored-C cleanliness pass.

## Decision

Accept the divide-and-conquer stable partition together with the pinned
single-caller dereference wrapper boundary. The partition mirrors C's reuse of
existing literal storage, removes one allocation per constructed clause on the
exact workload, preserves stable sign ordering, and avoids the rejected
quadratic movement. The wrapper annotation is retained only as the measured
code-generation companion that lets the independent allocation saving
transfer to the whole prover.

The accepted baseline becomes 8,800,386,737 instructions, or 1.674873 times
C. Bead `E_Rust_Port-j76.5.3` remains open because overall performance parity
is not yet complete.

## Raw profiles

```text
.artifacts/experiments/2026-07-24-017-in-place-clause-sign-partition/callgrind-candidate.out
.artifacts/experiments/2026-07-24-017-in-place-clause-sign-partition/callgrind-out-of-line-candidate.out
.artifacts/experiments/2026-07-24-017-in-place-clause-sign-partition/callgrind-forced-wrapper-candidate.out
.artifacts/experiments/2026-07-24-017-in-place-clause-sign-partition/callgrind-divide-candidate.out
```

## Reproduction

```bash
cargo build --locked --release --bin eprover \
  --target-dir target-wsl-290d-divide-clause-sign-partition
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-divide-candidate.out \
  target-wsl-290d-divide-clause-sign-partition/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-286-fused-diversity-traversal\release\eprover.exe `
  -CandidateExe .\target\native-290d-divide-clause-sign-partition\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-017-in-place-clause-sign-partition\native-divide-lusk.csv
```
