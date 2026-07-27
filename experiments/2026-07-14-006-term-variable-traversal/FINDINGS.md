# Borrowed Term Variable Traversal

## Question

Can `term_collect_variables` borrow a term's argument slice and clone only
non-ground children without changing cached-ground pruning, traversal order, or
the set of collected variable identities?

## Setup

- Baseline commit: `fd16455f` (`Classify PDTree query metadata once`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-006-term-variable-traversal/baseline-eprover`.
- Baseline SHA-256:
  `23546b845227e8169866c396c8b801daa5c757b1e044da2171f520bcd562d042`.
- Candidate SHA-256:
  `137b6f5c3f5b36b58b3b1c3dd1c6b70b9af1eaecd1944bcdba9619c6bce2db75`.
- Primary problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Falsification problems:
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p` and
  `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`.
- Common proof options:
  `--auto --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

Rebuild and rerun the focused measurements from the repository root:

```bash
cargo build --locked --release --bin eprover
bash experiments/2026-07-14-006-term-variable-traversal/benchmark.sh
bash experiments/2026-07-14-006-term-variable-traversal/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-006-term-variable-traversal/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The repository-wide gates used the freshly rebuilt Windows binary and native
WSL comparison:

```powershell
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Implementation

The old traversal called `argument_clones` for every visited non-variable
term. That allocated and copied the complete optional argument vector before
filtering cached-ground children. The candidate borrows `current.arguments()`
once, skips empty and cached-ground slots in place, and clones only handles
pushed onto the existing traversal stack.

Arguments are still pushed from left to right, retaining the old LIFO visit
order. The focused regression test combines a cached-ground child, a visible
free variable, and an uninitialized argument slot to cover all three branches.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `2.78` and a
candidate median of `2.73` user seconds, a 1.8% improvement. Median wall time
improved from `2.93` to `2.88` seconds, or 1.7%. Raw timings are retained at
`.artifacts/experiments/2026-07-14-006-term-variable-traversal/alternating-times.txt`.

Matched Callgrind runs execute `22,351,336,038` baseline instructions and
`21,146,493,887` candidate instructions. The reduction is `1,204,842,151`
instructions, or 5.39%. The profile removes the traversal-specific iterator
fold and reduces `Term::argument_clones` from `513,010,862` to `314,083,231`
instructions, allocator work from `1,108,766,705` to `976,979,936`, and
`_int_free` from `1,460,028,699` to `1,299,366,779`. Raw profiles are retained
as `callgrind-baseline.out` and `callgrind-candidate.out` in the ignored
experiment artifact directory.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 45.34 s | 43.10 s | Unsatisfiable |
| GEO288+1 | 51.78 s | 50.73 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated, and
`1,022,255` rewrite-step counters. GEO288 retains exact `10,215` processed,
`128,583` generated, `127,990` paramodulation, and `34,170` rewrite-step
counters. GEO288's allocation-sensitive non-redundant subcounter differs by
16, within the already documented pointer-order behavior. These single long
pairs are semantic falsification checks rather than independent timing
evidence.

The final 50-case differential report is
`.artifacts/e-compare/20260714-154145-475524/`. Its six mismatches are the
established BOO020/SWV851 resource behavior, resource-sensitive GEO288,
normalized LUSK6ext/sledgehammer output, and synthetic one-second LUSK6 limit
cases. HEN011 agrees in this run. No new status, proof, or output mismatch class
appears.

The final five-run native report is
`.artifacts/e-compare/20260714-155448-809075-benchmark/`. Its aggregate Rust/C
wall ratio is `3.551`. Rust LUSK6 wall/CPU medians are `2.500`/`2.71` seconds;
LUSK6ext medians are `5.771`/`6.28`. These absolute Rust medians improve over
the preceding report's `2.680`/`2.92` and `6.081`/`6.62`, respectively, while
the aggregate ratio worsens because C also ran faster. Treat the cross-binary
aggregate as load-sensitive, not as evidence against the exact-binary result.

## Falsification Checks

- All 46 focused `terms::termfunc` tests pass, including cached-ground,
  visible-variable, and sparse-argument traversal.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The full suite passes 4,058 library tests and 3 integration tests, with all
  binary targets clean.
- LUSK6 improves in alternating exact-binary pairs and deterministic
  instruction count.
- HEN011 and GEO288 preserve status and principal saturation counters.
- The 50-case report contains no new mismatch class.
- Both experiment scripts pass `bash -n` and resolve paths from their own
  experiment directory.

## Conclusion And Limits

Borrowing the argument slice is accepted because it removes a per-node vector
copy, substantially reduces deterministic instructions and allocator work,
and improves paired LUSK6 CPU time while preserving proof behavior. The
repository-wide native benchmark still shows a large Rust/C performance gap.

C's `TermCollectVariables` already reads `args` directly but allocates and
frees a generic `PStack` for every call. A later cleanup could reuse
caller-owned search scratch or use a small inline stack while retaining direct
argument access, left-to-right pushes, and cached-ground pruning. Rust still
allocates one `Vec` per collection call; eliminating or reusing that remaining
stack requires a separate ownership and concurrency review.
