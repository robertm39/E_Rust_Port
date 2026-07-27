# Rejected forced inline PD-tree search initialization

## Question

Does forcing `PdTree::record_search_init` into its hot eta-normalizing wrapper
improve the whole prover after the accepted PD-tree cursor work?

## Setup

- Parent source: commit `1bc474bb` (`Force-inline hot PD-tree frame pop`),
  accepted Experiment 203.
- Candidate: add only `#[inline(always)]` and a narrow Clippy expectation to
  `PdTree::record_search_init`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-203-force-inline-pdt-frame-pop/rust-callgrind-force-inline-pdt-frame-pop.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-204-force-inline-pdt-search-init/rust-callgrind-force-inline-pdt-search-init.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The accepted profile has 877,339 calls to `record_search_init`, all from the
measured `record_search_init_with_bank` wrapper. This makes the annotation a
bounded single-edge experiment despite the initialization body's moderate
size.

## Deterministic result

The candidate still proves LUSK6 with the expected unsatisfiable status, but
retires 11,069,467,015 instructions. This is 154,788,986 above the
10,914,678,029-instruction parent, a 1.418173% whole-prover regression. The
hypothetical C/Rust ratio worsens from 2.077261 to 2.106720.

The directly comparable initialization aggregate falls from 130,723,511 to
114,054,070 instructions, saving 16,669,441 or 12.751678%. The whole-program
regression therefore comes from broader compiler code-generation and layout
redistribution rather than extra work at the annotated boundary. This is
another case where a locally favorable inline transformation is globally
harmful.

## Validation

- All 41 focused PD-tree library tests pass.
- Formatting passes.
- The candidate reaches the expected LUSK6 proof and exit zero under
  Callgrind.
- Native and compatibility matrices were intentionally skipped after the
  deterministic whole-program gate failed by 1.42%.

## Decision

Reject and restore the parent source. The local call boundary becomes cheaper,
but the executable as a whole retires 154.8 million more instructions. Preserve
this result to avoid retrying `record_search_init` forced inlining.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-pdt-search-init.out \
  target-wsl-204-force-inline-pdt-search-init/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
