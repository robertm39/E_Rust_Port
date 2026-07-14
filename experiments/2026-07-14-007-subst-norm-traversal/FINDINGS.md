# Borrowed Substitution Normalization Traversal

## Question

Can `Substitution::norm_term` push borrowed arguments directly in reverse
without materializing, reversing, and consuming a cloned argument vector at
every non-variable node?

## Setup

- Baseline commit: `edfffd31` (`Avoid cloning ground term arguments`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-007-subst-norm-traversal/baseline-eprover`.
- Baseline SHA-256:
  `137b6f5c3f5b36b58b3b1c3dd1c6b70b9af1eaecd1944bcdba9619c6bce2db75`.
- Candidate SHA-256:
  `11bf2e6a246e3a3e1ba21ee5028ca4151011d12972fb8cfa38ca7ac389fe6091`.
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
bash experiments/2026-07-14-007-subst-norm-traversal/benchmark.sh
bash experiments/2026-07-14-007-subst-norm-traversal/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-007-subst-norm-traversal/callgrind-candidate.out \
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

The old traversal cloned the complete optional argument vector, reversed that
temporary vector, flattened it, and extended the traversal stack. The
candidate borrows `current.arguments()` and pushes each initialized handle
from right to left. The LIFO traversal therefore retains C's left-to-right
variable binding order while avoiding the per-node vector allocation and
iterator teardown.

The focused regression test now asserts the substitution binding stack is
exactly `[x, y]`, in addition to checking fresh marked bindings and
backtracking.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `2.73` and a
candidate median of `2.70` user seconds, a 1.1% improvement. Median wall time
improved from `2.92` to `2.85` seconds, or 2.4%. Raw timings are retained at
`.artifacts/experiments/2026-07-14-007-subst-norm-traversal/alternating-times.txt`.

Matched Callgrind runs execute `21,146,493,887` baseline instructions and
`20,273,920,949` candidate instructions. The reduction is `872,572,938`
instructions, or 4.13%. `Substitution::norm_term` self-cost falls from
`393,352,146` to `297,995,188` instructions. `malloc`, `_int_free`, and `free`
fall by another `141,920,319`, `173,185,138`, and `99,067,444` instructions,
respectively. The baseline call edge from `norm_term` into cloned-vector
extension accounted for `770,241,716` inclusive instructions and is absent
from the candidate implementation. Raw profiles are retained as
`callgrind-baseline.out` and `callgrind-candidate.out` in the ignored
experiment artifact directory.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 43.19 s | 42.72 s | Unsatisfiable |
| GEO288+1 | 50.81 s | 50.76 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated,
`1,062,557` paramodulation, and `1,022,255` rewrite-step counters. GEO288
retains exact `10,215` processed, `128,583` generated, `127,990`
paramodulation, `34,170` rewrite-step, and allocation-sensitive non-redundant
counters. These single long pairs are semantic falsification checks rather
than independent timing evidence.

The final 50-case differential report is
`.artifacts/e-compare/20260714-164059-092301/`. Its five mismatches are the
established BOO020/SWV851 resource behavior, resource-sensitive GEO288,
normalized LUSK6ext output, and synthetic one-second LUSK6 limit cases.
Sledgehammer agrees in this run. No new status, proof, or output mismatch class
appears.

The final five-run native report is
`.artifacts/e-compare/20260714-165236-803463-benchmark/`. Its aggregate Rust/C
wall ratio is `3.303`, down from the preceding report's `3.551`. Rust LUSK6
wall/CPU medians improve from `2.500`/`2.71` to `2.392`/`2.60` seconds;
LUSK6ext improves from `5.771`/`6.28` to `5.738`/`6.25`. The native report
confirms the direction of the exact-binary result, while cross-binary ratios
remain sensitive to C-side timing and the resource-capped corpus.

## Falsification Checks

- All 9 focused `terms::subst` tests pass, including exact variable-binding
  order.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The full suite passes 4,058 library tests and 3 integration tests, with all
  binary targets clean.
- LUSK6 improves in alternating exact-binary pairs and deterministic
  instruction count.
- HEN011 and GEO288 preserve status and compared saturation counters exactly.
- The 50-case report contains no new mismatch class.
- Both experiment scripts pass `bash -n` and resolve paths from their own
  experiment directory.

## Conclusion And Limits

The borrowed reverse traversal is accepted because it eliminates a hot
per-node vector allocation, reduces deterministic instructions by 4.13%, and
improves exact-binary and native LUSK6 timing while preserving binding order
and proof behavior.

C's `SubstNormTerm` already uses a 64-slot inline `PLocalStack` and direct
reversed argument pushes. Its `Sig_p sig` parameter is unused, while the
dereference strategy is selected through process-global `problemType`. A
later cleaned API could remove the unused signature and accept an explicit
problem type or dereference strategy without sacrificing the current stack
and raw-argument performance. Rust still allocates one traversal `Vec` per
normalization call; reusing that remaining stack requires a separate
reentrancy and substitution-ownership review. Rust's current `norm_term`
signature also lacks the mutable `TermBank` required by `whnf_deref`, so it
retains generic `term_deref(Always)` in higher-order mode where C selects
`WHNF_deref`. Closing that compatibility gap requires a separate caller and
term-bank ownership change.
