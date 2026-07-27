# Experiment 256: Evaluation-splay line attribution

## Status

Completed as a diagnostic in Experiment 256 for Bead
`E_Rust_Port-j76.5.3`.

## Question

What source operations dominate the accepted 306,825,308-instruction
`EvalIndexTree::splay`, after direct sentinel links and the surrounding direct
tree boundary are already retained?

The remaining structural difference from C is safe indexed arena access.
This diagnostic measures unchanged accepted source before deciding whether a
narrow unchecked-access candidate is justified under the repository's unsafe
Rust policy.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851
- accepted evaluation splay: 306,825,308 exclusive instructions

## Profile

The accepted source was rebuilt with release optimization and line tables:

```text
CARGO_PROFILE_RELEASE_DEBUG=1
CARGO_TARGET_DIR=target-wsl-256-eval-splay-lines
cargo build --release --locked --bin eprover
```

The line-table binary proves LUSK6 exactly and retires 9,897,779,299
instructions. That is 655,467 instructions, or 0.006622%, below the accepted
baseline, so the profile is representative. The evaluation splay reproduces
exactly at 306,825,308 exclusive instructions.

Aggregating direct costs by attributed source gives:

| Source attribution | Instructions | Splay share |
| --- | ---: | ---: |
| `src/clauses/clausesets.rs` | 159,685,994 | 52.044597% |
| `core::slice::index` | 101,614,782 | 33.118123% |
| `core::cmp` | 43,670,654 | 14.233068% |
| `core::option` | 1,853,878 | 0.604213% |

The standard-library slice costs are entirely the bounds-checked node access
boundary:

- immutable indexing at `core::slice::index:272`: 86,659,195 instructions,
  or 28.243822% of the splay;
- mutable indexing at `core::slice::index:278`: 14,955,587 instructions, or
  4.874300% of the splay.

The comparator cost is already contiguous in each 48-byte node. Prior
experiments rejected comparator restatements, a separate cold key arena,
returning the terminal ordering, and forced splay inlining. Direct sentinel
links and the direct outer tree boundary are already retained.

## Result

Production source is unchanged. The profile isolates safe arena indexing as
the largest remaining Rust-specific splay cost and supplies the concrete
performance reason required before using unsafe Rust.

The next bounded candidate will keep the public and operation boundary safe,
but use unchecked node access only inside `EvalIndexTree::splay`. Its safety
argument must cover the valid input root, the sentinel check before every
child dereference, the invariant that every live child link names an allocated
arena slot, the absence of arena growth during splay, and the fact that each
mutable node borrow ends before the next access. Existing safe access remains
unchanged outside the splay.

The raw profile is preserved at:

```text
.artifacts/experiments/2026-07-23-018-eval-splay-line-attribution/rust-callgrind-eval-splay-lines.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-eval-splay-lines.out \
  target-wsl-256-eval-splay-lines/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
