# Experiment 257: Unchecked evaluation-splay node access

## Status

Rejected in Experiment 257 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

Experiment 256 attributes 101,614,782 instructions, or 33.118123% of the
accepted 306,825,308-instruction `EvalIndexTree::splay`, to bounds-checked
slice indexing. C traverses the same top-down splay through direct pointers.

Keep the splay entry point safe and keep every operation outside the splay on
the existing checked node access. Inside the splay only, use private unchecked
node access after documenting and debug-checking these invariants:

- the input root is a live arena index;
- every live child link is a valid arena index;
- the null sentinel is checked before every child dereference;
- the arena cannot grow or shrink during splay;
- each mutable node borrow ends before another node access.

Preserve the 48-byte node, direct sentinel links, comparator, rotation and
reassembly order, free-slot reuse, duplicate handling, and outer tree API.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851
- accepted evaluation splay: 306,825,308 exclusive instructions

## Candidate

Add two private unchecked node-access primitives used only by the existing
safe `splay` method. Each primitive debug-checks its live-slot precondition,
documents its `# Safety` contract, and contains one locally allowed unsafe
slice access. The splay has one documented unsafe scope covering root/link
validity, sentinel checks, no arena resizing, and non-overlapping
statement-local borrows.

All checked access outside the splay remains unchanged. The candidate adds no
raw pointers, pointer arithmetic, representation change, or topology change.

## Validation

- The focused evaluation-index topology, ordering, duplicate, removal, and
  free-slot-reuse test passes with every unsafe index debug-checked.
- Strict library pedantic Clippy passes, including unsafe documentation.
- Formatting and `git diff --check` pass.
- The exact LUSK6 profile proves `Unsatisfiable` and exits zero.
- Direct native parent and candidate output is byte-exact; both contain the
  proof and SZS success markers and exit zero.
- All 128 measured native processes exit zero.

## Deterministic measurement

The candidate retires 9,857,237,388 instructions, 41,197,378 below the
9,898,434,766-instruction parent. This is a 0.416201% whole-prover improvement,
and the hypothetical Rust/C ratio changes from 1.883851 to 1.876011.

`EvalIndexTree::splay` falls from 306,825,308 to 266,646,691 exclusive
instructions:

- delta: -40,178,617;
- local improvement: -13.094949%;
- the intended local boundary explains 97.527% of the whole-program
  reduction.

The Windows candidate binary shrinks 2,560 bytes, from 8,654,336 to 8,651,776
bytes.

## Native production measurement

After a byte-exact direct proof check and four alternating warmup pairs, one
independent block ran 64 alternating parent/candidate pairs with a fresh
process for each execution.

Across all 64 pairs, the candidate regresses mean paired wall time by
2.335357% and CPU time by 1.763567%. Median paired wall and CPU changes regress
1.261960% and 1.075269%; aggregate wall and CPU time regress 2.282600% and
1.707437%. The candidate wins only 16 wall pairs and 15 CPU pairs, with seven
CPU ties.

The stable last 32 pairs remain negative:

- mean paired wall time: +2.613931%;
- mean paired CPU time: +1.773380%;
- median paired wall time: +1.146217%;
- median paired CPU time: +1.063830%;
- aggregate wall time: +2.576447%;
- aggregate CPU time: +1.746293%;
- wins: 7 wall and 7 CPU, with 4 CPU ties.

The stable native regression is far larger than the deterministic gain, so a
second block is unnecessary.

## Result

Reject. Remove all unsafe code and restore bounds-checked node access
byte-for-byte. The concrete profile justified testing the contained unsafe
implementation, but the production result eliminates the performance reason
required to retain it under the repository policy.

Accepted Experiment 245 remains the baseline at 9,898,434,766 instructions,
or 1.883851 times C. Compatibility and resource matrices are skipped after
the decisive native production failure.

The measured native samples are in `native-lusk.csv`. Raw ignored artifacts
are preserved at:

```text
.artifacts/experiments/2026-07-23-019-unchecked-eval-splay-nodes/rust-callgrind-unchecked-eval-splay-nodes.out
.artifacts/experiments/2026-07-23-019-unchecked-eval-splay-nodes/native-warmup.csv
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-unchecked-eval-splay-nodes.out \
  target-wsl-257-unchecked-eval-splay-nodes/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
