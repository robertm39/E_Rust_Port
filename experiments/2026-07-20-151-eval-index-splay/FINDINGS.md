# Compact splay evaluation index

## Question

Can clause-selection evaluation indices replace Rust's generic `BTreeSet`
with the same top-down splay organization as C's `EvalTree`, while preserving
the Rust port's distinct-object tie-break, exact clause-selection order, and
the maintained Windows resource boundary?

## Setup

- Parent source: commit `c5cba85c` (`Reduce term-tree comparator ownership
  overhead`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 14,191,666,721 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained at
`.artifacts/experiments/2026-07-20-151-eval-index-splay/`. Compatibility
reports are retained under `.artifacts/e-compare/`.

## Tree replacement

Each clause set previously held one `BTreeSet<EvalIndexEntry>` per evaluation
position. The generic B-tree insertion path was the profile's seventh-largest
exclusive function at 440,928,436 instructions. C instead uses a top-down
splay tree and finds the leftmost cell for best-clause selection.

The retained implementation uses a private, non-recursive top-down splay set.
Nodes live in an indexed arena and deleted slots are reused, avoiding one heap
allocation per tree node. In-order iteration preserves the existing debug and
test accessors, while `first`, duplicate insertion, and keyed removal preserve
the former set contract. The comparator is unchanged: priority, heuristic,
and evaluation age follow C ordering, and the Rust object handle remains the
final tie-break so cloned evaluation cells cannot collapse into one entry.

Direct tests cover sorted iteration, best-entry lookup, duplicate suppression,
successful and missing removal, freed-slot reuse, logical equality across
different splay shapes, clearing, and cloned-cell distinctness.

## Resource-safe node layout

The first arena draft used native `Option<usize>` links and an outer
`Option<Node>` tombstone. It produced the exact proof at 14,064,470,214
instructions, 127,196,507 below the parent (-0.8963%), but failed the resource
gate. Report `.artifacts/e-compare/20260720-175252-276926/` records BOO020
aborting on the known 139,264-byte clause-page allocation at 53.16 seconds;
SWV851 remained exact.

The final arena stores links as encoded `Option<NonZeroUsize>` values and
keeps stale `Copy` node bits in recycled slots instead of wrapping every node.
The live node is therefore exactly one evaluation key plus two pointer-width
links (48 bytes on the maintained 64-bit target), an invariant pinned by the
unit test. This compact form produces the exact proof at 14,023,295,072
instructions, 168,371,649 below the parent (-1.1864%) and 41,175,142 below the
first draft (-0.2928%). `EvalIndexTree::splay` costs 368,087,119 exclusive
instructions, while the old B-tree insertion alone cost 440,928,436.

The direct C profile remains 5,254,361,329 instructions, so the deterministic
Rust/C ratio improves from about 2.70 to 2.669. The final profile is retained
as `rust-callgrind-eval-splay-compact.out`; the rejected oversized profile is
retained as `rust-callgrind-eval-splay.out`.

## Compatibility result

Three consecutive isolated compact-arena BOO020 reports have zero mismatches:

- `.artifacts/e-compare/20260720-180650-015324/`
- `.artifacts/e-compare/20260720-181310-370301/`
- `.artifacts/e-compare/20260720-181524-864170/`

The final combined BOO020/SWV851 resource report at
`.artifacts/e-compare/20260720-181746-214113/` also has zero mismatches. The
final four-case proof report at
`.artifacts/e-compare/20260720-182650-021816/` has zero mismatches across
GEO288, HEN011, LUSK6, and LUSK6ext.

The complete Rust suite passes 4,370 unit tests plus every integration target.
Strict all-target/all-feature pedantic Clippy, formatting, and the all-feature
release build pass.

## Falsification checks

- Exact deterministic and native proof output rules out a gain caused by a
  changed clause-selection order.
- The distinct-object regression keeps cloned evaluation cells separately
  selectable even though C would treat equal cell keys as a collision.
- The rejected large-node resource report demonstrates that exact proof and
  lower instruction count alone were insufficient acceptance evidence.
- Four exact compact-arena BOO020 runs, including the combined resource run,
  guard against the known intermittent Windows boundary failure.
- The retained tree is iterative, so adversarial splay depth cannot overflow
  the Rust call stack.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-eval-splay-compact.out \
  target-wsl-151-eval-splay-compact/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-151-eval-splay
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-151-eval-splay\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-151-eval-splay\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept the compact evaluation-index splay set. It moves clause selection closer
to C's data structure, removes the generic B-tree insertion hotspot, preserves
the Rust clone-distinctness contract, and passes the proof and repeated resource
gates. Keep the main parity issue open: the synthetic one-second LUSK cutoff
and the remaining 2.669 deterministic C/Rust instruction ratio still fail the
project-wide acceptance criteria.
