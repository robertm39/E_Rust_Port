# Experiment 289: Reuse allocator size classification

## Status

Rejected in Experiment 289 for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the exact-size allocator reuse the cacheability result already computed
for its locked free-list probe when it constructs the backing `System`
allocation layout on a cache miss?

## Candidate

The accepted allocator called `cacheable_size(layout)` before probing a free
list and called it again after a cache miss. The candidate carries the first
successful size classification through to `cached_layout(size)`. It preserves
the exact size classes, 16-byte backing alignment, global lock boundary,
intrusive list operations, failure flush, and retry policy.

## Baseline

- Accepted source: commit `6d0ce72a` (Experiment 286).
- Accepted exact LUSK6 Callgrind: 8,828,399,104 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Accepted Rust/C ratio: 1.680204.

## Validation and measurement

The candidate passes:

- the four focused all-feature allocator tests, including parallel reuse;
- strict all-feature library pedantic Clippy;
- formatting and diff checks;
- three accepted-parent and eight candidate native proof-output runs, all
  exiting zero with identical 378-character `Unsatisfiable` output and SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

The default-feature Callgrind run preserves the exact proof and retires
8,824,394,281 instructions:

- delta: -4,004,823 instructions;
- improvement: 0.045363%;
- Rust/C ratio: 1.679442.

The candidate and accepted parent Windows executables are both 8,964,608
bytes. After a four-pair warmup, two independent 64-pair alternating blocks
give mixed native results.

| Window | Wall mean | CPU mean | Paired wall mean | Paired CPU mean |
| --- | ---: | ---: | ---: | ---: |
| Block 1 | +0.698244% | -0.234954% | +0.762977% | -0.136344% |
| Block 2 | +0.063865% | -0.092490% | +0.121250% | -0.032871% |
| Combined | +0.385056% | -0.164549% | +0.442114% | -0.084607% |

Positive values are regressions. Across all 128 pairs, the candidate wins 60
wall pairs and 55 CPU pairs, with 20 CPU ties. Its paired wall median regresses
0.106938%; the paired CPU median is exactly tied. The combined last halves
improve wall and CPU means by 0.097038% and 0.240785%, while the combined final
quarters improve them by 0.181447% and 0.369959%. These late windows show that
the candidate is close to native-neutral, not that it produces a replicated
production improvement.

The raw deterministic profile is retained at:

```text
.artifacts/experiments/2026-07-24-016-reuse-allocator-classification/callgrind-candidate.out
```

The measured native rows are retained in `native-lusk.csv` and
`native-lusk-2.csv`.

## Decision

Reject. Removing the duplicate classification saves 0.045363% deterministic
instructions, but the full replicated native wall evidence regresses and CPU
evidence is effectively neutral. That is not enough production evidence to
change the unsafe global-allocation boundary for a micro-optimization.

Restore the Experiment 286 allocator byte-for-byte. The accepted baseline
remains 8,828,399,104 instructions, or 1.680204 times C. The global exact-size
free list remains valuable, but its tested lock, empty-list, thread-local,
per-class-lock, and classification-reuse refinements are now exhausted.

## Reproduction

```bash
cargo build --locked --release --bin eprover \
  --target-dir target-wsl-289-reuse-allocator-classification
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-candidate.out \
  target-wsl-289-reuse-allocator-classification/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-286-fused-diversity-traversal\release\eprover.exe `
  -CandidateExe .\target\native-289-reuse-allocator-classification\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-016-reuse-allocator-classification\native-lusk.csv
```
