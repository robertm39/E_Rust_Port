# Borrowed Term-Top Comparison

## Question

Can `term_top_compare_for_problem` borrow both argument arrays once and compare
their term identities by reference without changing C-compatible key order or
splay-tree behavior?

## Setup

- Baseline commit: `47b65cff` (`Avoid cloning substitution traversal arguments`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-008-term-top-compare-borrows/baseline-eprover`.
- Baseline SHA-256:
  `11bf2e6a246e3a3e1ba21ee5028ca4151011d12972fb8cfa38ca7ac389fe6091`.
- Candidate SHA-256:
  `b59290f051c2869062f0e195a22a8601bdcdfced237854bb1449bddee89b159c`.
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
bash experiments/2026-07-14-008-term-top-compare-borrows/benchmark.sh
bash experiments/2026-07-14-008-term-top-compare-borrows/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-008-term-top-compare-borrows/callgrind-candidate.out \
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

The old comparator queried arity repeatedly and called `Term::argument` for
both operands at every index. Each call acquired a new `RefCell` borrow and
cloned the selected `Rc` term handle. The candidate holds one immutable
argument-slice borrow per operand, compares their lengths, and passes borrowed
argument handles directly to `term_identity_cmp`.

Function-code, first-order type assertion, higher-order type-address ordering,
arity ordering, first differing argument, and uninitialized-argument panic
behavior are unchanged. The focused test now asserts the exact argument
identity comparison result rather than only checking it is nonzero.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `2.69` and a
candidate median of `2.63` user seconds, a 2.2% improvement. Median wall time
improved from `2.84` to `2.75` seconds, or 3.2%. Raw timings are retained at
`.artifacts/experiments/2026-07-14-008-term-top-compare-borrows/alternating-times.txt`.

Matched Callgrind runs execute `20,273,920,949` baseline instructions and
`20,023,941,923` candidate instructions. The reduction is `249,979,026`
instructions, or 1.23%. `term_top_compare_for_problem` alone falls from
`972,074,340` to `722,097,577`, a `249,976,763` instruction reduction that
accounts for effectively the complete program-level change. Raw profiles are
retained as `callgrind-baseline.out` and `callgrind-candidate.out` in the
ignored experiment artifact directory.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 44.55 s | 42.27 s | Unsatisfiable |
| GEO288+1 | 50.60 s | 51.26 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated,
`1,062,557` paramodulation, and `1,022,255` rewrite-step counters. GEO288
retains exact `10,215` processed, `128,583` generated, `127,990`
paramodulation, and `34,170` rewrite-step counters. GEO288's
allocation-sensitive non-redundant subcounter differs by two, within the
already documented raw-address ordering behavior. The opposite timing
directions make these single pairs semantic checks, not timing evidence.

The final 50-case differential report is
`.artifacts/e-compare/20260714-173418-752106/`. Its six mismatches are the
established BOO020/SWV851 resource behavior, resource-sensitive GEO288,
normalized LUSK6ext/sledgehammer output, and synthetic one-second LUSK6 limit
cases. No new status, proof, or output mismatch class appears.

The final five-run native report is
`.artifacts/e-compare/20260714-174606-416345-benchmark/`. Its aggregate Rust/C
wall ratio is `3.330` versus `3.303` in the preceding report, with C-side and
cross-corpus variation dominating that aggregate. Rust LUSK6 wall/CPU medians
improve from `2.392`/`2.60` to `2.325`/`2.52` seconds; LUSK6ext improves from
`5.738`/`6.25` to `5.351`/`5.83`. The relevant native medians confirm the
exact-binary direction despite the noisy aggregate ratio.

## Falsification Checks

- All 4 focused `terms::termtrees` tests pass, including exact argument
  identity ordering and higher-order type-address ordering.
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

The borrowed comparator is accepted because it removes repeated `RefCell`
borrows and `Rc` clones from a hot pure comparison, isolates a 1.23%
deterministic instruction reduction to that function, and improves paired and
native LUSK6 timing without changing pointer-key behavior.

C's comparator documentation still describes a masked-properties key that the
body no longer implements; the real key is function code, higher-order type
address when applicable, arity, and argument addresses. C's `uintptr_t`
address order and Rust's allocation-identity order make splay-tree shape
process-local and allocator-sensitive. A later stable-ID key could improve
reproducibility, but only after proof/output traces and performance no longer
depend on the current allocation order.
