# C-shaped PD-tree function alternatives

## Question

Can Rust stop duplicating every PD-tree function edge in an ordered
`PrefixToken` map and use the already represented C `IntMap` as the actual
function-code-to-child index without changing proof order, higher-order edge
handling, or the maintained BOO020 resource boundary?

## Setup

- Parent source: commit `0b4c78db` (`Make clause insertion reservations
  fallible`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,778,448,460 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained profile is
`.artifacts/experiments/2026-07-21-166-pdt-function-intmap/rust-callgrind-intmap.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Structural attribution

C `PDTNodeCell` stores ordinary function children directly in
`f_alternatives`, an `IntMap` keyed by `f_code`; free-variable and DB-like
alternatives live in separate object maps. Rust already maintained an
`IntMap<()>` per node to reproduce C storage accounting, but stored the actual
child index for every edge in a second `BTreeMap<PrefixToken, usize>`. The
production first-order cursor therefore paid an ordered enum-key lookup even
though the C-shaped integer representation had already been built.

The parent profile charges 201,590,330 exclusive instructions to the standard
library `BTreeMap<PrefixToken, usize>` search routine. Together with the
1,603,016,018-instruction PD-tree cursor, this makes the duplicated lookup a
larger target than another traversal-frame micro-optimization. Earlier compact
frame work was also rejected because its extra throughput reopened the BOO
allocator boundary.

## Accepted representation

`PdNode::fun_alternatives` now maps a function code directly to its child node
index. The ordered `children` map contains only free-variable and DB-like
tokens. One child-dispatch helper selects the same split for insertion,
deletion, prefix matching, compatibility collection, path probing, and the
production lazy cursor. Constraint recomputation visits both maps.

The shared `IntMap` gains an immutable lookup that checks existing array
coverage instead of reproducing C's incidental range-array growth on a miss
below the current offset. The original mutable `get_val` remains unchanged for
compatibility callers. A focused regression pins the immutable no-growth
contract, while PD-tree storage tests verify that ordinary function edges no
longer appear in the object map and still preserve the existing C-shaped
storage estimate.

## Performance result

The candidate preserves the exact 4,873-clause LUSK6 proof at
12,625,510,206 instructions. This is 152,938,254 below the parent (-1.1968%),
improving the deterministic C/Rust ratio from 2.432 to 2.403. The standalone
`BTreeMap<PrefixToken, usize>` search hotspot disappears. Inlined integer-map
dispatch raises the cursor's exclusive total from 1,603,016,018 to
1,650,291,596 instructions, but the combined former cursor-plus-map cost falls
by 154,314,752 instructions, matching the end-to-end reduction closely.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-070953-329454/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-071157-113851/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`
  instead of aborting in the allocator.
- Full report `.artifacts/e-compare/20260721-071612-277953/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. BOO, SWV, HEN, and the synthetic one-second LUSK case all match.

## Validation

- `cargo fmt --all -- --check`
- 4,379 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept the split function/object child representation. It restores C's actual
PD-tree function-alternative ownership instead of maintaining `IntMap` as an
accounting shadow, removes the hot ordered enum-key lookup, improves exact
whole-prover instructions by 1.1968%, and passes the complete compatibility
and constrained-resource matrix. Keep the main performance issue open: the
remaining deterministic C/Rust instruction ratio is 2.403, still far above
the project requirement.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-intmap.out \
  target-wsl-166-pdt-intmap/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-166-pdt-intmap
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-166-pdt-intmap\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-166-pdt-intmap\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
