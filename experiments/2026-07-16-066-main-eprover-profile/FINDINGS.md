# Main Eprover KBO6 Traversal Profile

## Question

Can a deterministic profile of the main prover identify allocation work that
can be removed without changing proof search, and does the resulting change
improve both a controlled prefix and full long-running proofs?

## Setup

- Baseline commit: `2b616f7e` (`Resolve PCL proofcheck compatibility edges`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-16-066-main-eprover-profile/baseline-eprover`.
- Baseline SHA-256:
  `2a1a6db37b0356dd1c74dd9b70620f3b5722123f71285b2398fec81545ac5af6`.
- Candidate Linux SHA-256:
  `911faaa5ac09d96d79c161baffda8a2e8d587970daa0a27170c1c496847a6e67`.
- Primary profile problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Controlled-prefix problem:
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p` with
  `--processed-clauses-limit=50000`.
- Full falsification problems:
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p` and
  `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`.
- Common proof options:
  `--auto --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

Rebuild and rerun the focused measurements from the repository root:

```bash
cargo build --locked --release --bin eprover
bash experiments/2026-07-16-066-main-eprover-profile/benchmark.sh
bash experiments/2026-07-16-066-main-eprover-profile/hen-prefix-benchmark.sh
bash experiments/2026-07-16-066-main-eprover-profile/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-16-066-main-eprover-profile/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The repository-wide validation used the freshly rebuilt Windows executable and
native WSL benchmark:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Profile And Accepted Change

The baseline LUSK6 Callgrind run executes `20,073,916,355` instructions.
`Term::argument_clones` accounts for `392,429,535` instructions and
`6,051,647` calls. One caller is the iterative `mfy_vwb` KBO6
variable-balance walker, which makes `1,306,910` calls into that helper.

C's corresponding walker uses a local pointer stack and pushes borrowed raw
argument-array entries. Rust now borrows the term's argument slice and clones
only the individual shared `Term` handles pushed onto its explicit stack. This
removes construction and cloning of a temporary argument vector at every
visited node while retaining the same reverse stack-push order, dereference
policy, weights, and balance updates.

The accepted candidate executes `19,899,749,157` instructions: `174,167,198`
fewer than the exact baseline, a `0.87%` reduction. It retains the same proof
status and path.

## Timing Results

Seven alternating LUSK6 pairs were noisy. Median user time is `3.41` seconds
for the baseline and `3.43` for the candidate, while median wall time moves
from `3.62` to `3.50` seconds. Maximum RSS is effectively unchanged at about
`241` MiB. Raw results are retained in
`.artifacts/experiments/2026-07-16-066-main-eprover-profile/alternating-times.txt`.

Five alternating 50,000-processed-clause HEN011 prefix pairs give a more
controlled throughput result:

| Metric | Baseline median | Candidate median | Change |
| --- | ---: | ---: | ---: |
| User CPU | 5.95 s | 5.52 s | -7.23% |
| Wall | 5.70 s | 5.21 s | -8.60% |
| Maximum RSS | about 186.6 MiB | about 186.9 MiB | effectively flat |

One candidate wall sample was an `8.66`-second host-load outlier; the median is
unchanged if the raw results are inspected in
`.artifacts/experiments/2026-07-16-066-main-eprover-profile/hen-prefix-times.txt`.
Every run reached the same processed-clause limit with exit status 9.

The single full-proof pairs were deliberately candidate-first to reduce the
risk of accepting a baseline-first cache advantage. Consequently, the raw
`proof-checks` filenames have inverted executable labels: `*-baseline` is the
candidate and `*-candidate` is the saved baseline.

| Problem | Baseline user | Candidate user | Candidate change |
| --- | ---: | ---: | ---: |
| HEN011-2 | 64.04 s | 62.47 s | -2.45% |
| GEO288+1 | 55.70 s | 57.98 s | +4.09% |

Both candidates retain the exact theorem status and principal proof counters.
The mixed single-pair result prevents a broad throughput claim; the
deterministic instruction reduction and controlled HEN prefix govern the
narrow acceptance.

## Rejected Variants

The same profile supported four falsification experiments. Their raw timing,
Callgrind, and long-proof artifacts are retained under
`.artifacts/experiments/2026-07-16-066-main-eprover-profile/`.

- Replacing PDTree enter/exit frames with a reverse span pass increased
  instructions to `20,111,930,610` (`+0.19%`) and was rejected.
- Returning immediately when a PDTree query root was absent increased
  instructions to `20,365,496,408` (`+1.45%`) and was rejected.
- Removing the recursive rewrite preflight reduced instructions to
  `19,621,723,750` (`-2.25%`) and improved LUSK6, but made both HEN011 and
  GEO288 about 6.7% slower in paired full proofs. It was rejected.
- Borrowing argument slices across recursive rewriting reduced instructions to
  `19,888,899,570` (`-0.92%`), but made HEN011 about 10.4% and GEO288 about
  6.2% slower in paired full proofs. Holding dynamic borrows across recursion
  is therefore rejected for this path.

None of the rejected source variants remains in the worktree.

## Repository-Wide Validation

- All 19 focused KBO6 tests pass.
- `cargo test --all-targets --all-features` passes, including all 4,192 library
  tests and every binary/integration target.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The locked release `eprover` build and all 32 interoperability harness tests
  pass.
- All four C-source documentation checks pass.
- The full 50-case report at
  `.artifacts/e-compare/20260717-002556-450711/` retains exactly the six known
  differences: resource/cutoff behavior for BOO020, GEO288, HEN011, and the
  synthetic one-second LUSK6 case, plus normalized output for LUSK6ext and
  sledgehammer.
- The five-run report at
  `.artifacts/e-compare/20260717-003817-240716-benchmark/` measures a
  load-sensitive `3.148x` aggregate Rust/C wall ratio. LUSK6 is `2.722x`
  (`3.117` versus `1.145` seconds) and LUSK6ext is `2.603x` (`7.116` versus
  `2.734` seconds). This is worse than the pre-change report's `3.032x`
  aggregate despite the focused deterministic improvement.

## Conclusion And Limits

Borrowing the KBO6 argument slice removes a temporary vector and redundant
shared-handle clones from a million-call traversal without changing ordering
semantics. The deterministic profile and controlled HEN prefix show less work
and better throughput, while full proof counters and the compatibility corpus
remain stable.

This change does not establish overall performance parity. Full-proof timing
is workload- and load-sensitive, the standard suite remains roughly three
times slower than C, and the accepted change leaves other high-cost term,
rewrite, discrimination-tree, allocation, and higher-order paths untouched.
