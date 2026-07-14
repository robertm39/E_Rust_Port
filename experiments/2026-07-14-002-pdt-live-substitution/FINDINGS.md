# PDTree Live-Substitution Cursor

## Question

Can first-order demodulator lookup match C's incremental
`PDTreeFindNextDemodulator` contract by retaining the accepted match
substitution, instead of collecting every candidate and matching each one again,
without regressing the long GEO288 or HEN011 proof searches?

## Setup

- Baseline commit: `a48ba868` (`Reduce first-order matching stack overhead`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-002-pdt-live-substitution/baseline-eprover`.
- Candidate: WSL release `target/release/eprover`.
- Primary problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Falsification problems:
  `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p` and
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p`.
- Common proof options:
  `--auto --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted candidate can be rebuilt and timed from the repository root:

```bash
cargo build --locked --release --bin eprover
bash experiments/2026-07-14-002-pdt-live-substitution/benchmark.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-002-pdt-live-substitution/callgrind-arena.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The final repository-wide gates were run from normal Windows PowerShell:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Candidate Sweep

1. A direct continuation cursor bound and unbound the shared `Substitution`
   while repeatedly entering `BTreeMap::range`. It raised LUSK6 Callgrind
   instructions from `23,973,870,356` to `25,310,829,558` and regressed
   alternating user time by about 2.3%, so it was rejected.
2. Reusing frames and keeping speculative bindings in a cursor-local vector
   lowered instructions to `23,566,769,253`. A `Vec` of variable children
   in every node still regressed LUSK6 user time by about 1.3%, so that node
   layout was rejected.
3. Two packed links in every node removed the side allocation and made LUSK6
   roughly neutral, but a paired HEN011 run was `50.08` versus `47.68`
   user seconds. Enlarging every hot node was rejected.
4. A sparse `BTreeMap` keyed only by parents with variable children restored
   node size but made LUSK6 `3.36` versus `3.14` median user seconds because
   every cursor step paid a tree lookup. It was rejected.
5. The accepted layout keeps a separate four-byte head table indexed by PDTree
   node and an arena containing only real variable edges. Arena slots are
   recycled after deletion. This leaves `PdNode` unchanged, gives direct
   cursor traversal, and stores shared variable handles only where needed.

## Results

The accepted arena profile executes `23,498,629,423` LUSK6 instructions,
`475,240,933` fewer than the exact baseline, a 1.98% reduction.

Seven interleaved LUSK6 pairs measured a baseline median of `3.13` and a
candidate median of `3.10` user seconds. One host-load outlier affected each
series, so the deterministic instruction count is the stronger result.

The paired long-search checks were effectively neutral:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 48.28 s | 48.43 s | Unsatisfiable |
| GEO288+1 | 52.48 s | 53.34 s | Theorem |

HEN011 retained the principal `265,284` processed, `1,062,557` generated,
and `1,022,255` rewrite-step counters. GEO288 retained `10,215` processed,
`128,583` generated, `127,990` paramodulations, and `34,170` rewrite
steps. Small final subsumption/unprocessed-count differences remain consistent
with already documented allocation/order sensitivity.

The final 50-case differential report is
`.artifacts/e-compare/20260714-100918-853243/`. It has six mismatches, down
from the previous seven because GEO288 now matches, with no new mismatch:

- Resource or status behavior: BOO020, HEN011, SWV851, and the synthetic
  one-second LUSK6 case.
- Normalized output only: LUSK6ext and sledgehammer.

The final five-run native report is
`.artifacts/e-compare/20260714-102212-900104-benchmark/`. Its aggregate
Rust/C wall-time ratio is `3.486`, improved from `3.608`; LUSK6 is
`3.009` and LUSK6ext is `2.972`. General performance parity remains open.

## Falsification Checks

- All 31 PDTree tests pass, including repeated-variable rejection, live
  substitution lifetime, traversal order, and variable-edge deletion/reuse.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- Each accepted HEN011 and GEO288 run reaches the same proof status and
  principal proof-search counters as its saved baseline.
- The previous explicit cursor's roughly 10% GEO288 regression did not recur.
- The complete compatibility corpus adds no mismatch and closes GEO288 in the
  latest run.
- The benchmark script resolves paths from its own experiment directory and
  runs successfully from the repository root.

## Conclusion And Limits

A first-order demodulator candidate now arrives with its PDTree match bindings
still live, so rewriting does not perform the same full match twice. The compact
variable-edge arena is the only tested cursor layout that reduces deterministic
work without a material long-search regression.

Higher-order demodulator lookup intentionally retains the materialized,
bank-aware fallback because its matching and type side effects are not covered
by this first-order cursor. C's node-global traversal fields, reusable per-node
variable stacks, process-global traversal order, and raw-address leaf ordering
remain compatibility-relevant cleanup candidates; the paired C source review
documents why they should change only after drop-in compatibility is secured.
