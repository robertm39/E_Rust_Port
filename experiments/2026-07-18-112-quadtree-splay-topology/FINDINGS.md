# QuadTree splay topology

## Status

Completed for Bead `E_Rust_Port-j76.2.26`. Rust `QuadTree` now preserves the
top-down splay topology and locality behavior of `clb_quadtrees` with safe
index-linked storage. The vendored C checkout remains unchanged.

## Ownership and ordering audit

The only production C owner is the LPO-style comparison cache. It stores two
term pointers plus their dereference tags, canonicalizes symmetric queries, and
splays the requested or nearest key on every lookup. Rust has the same sole
owner in `cto_cmpcache`; its term identity is the allocation address obtained
from `Rc::as_ptr`. Thus both implementations compare native allocation
addresses without claiming that numeric addresses are stable across processes.
The addresses determine cache identity and locality only, never proof output.

The previous Rust implementation used `BTreeMap` and tracked only a synthetic
root key after successful operations. The replacement uses an arena of optional
nodes plus indices for the root and child links. It implements C's exact
top-down rotations, assembly, miss splaying, duplicate-preserving insertion,
extraction join, deletion, and freed-slot reuse without raw owning pointers.
Sorted iteration remains available through an in-order index walk.

## Exact topology evidence

[`capture_c_topology.py`](capture_c_topology.py) compiles the standalone
[`probe_quadtrees.c`](probe_quadtrees.c) against the pinned, unchanged C
`BASICS.a`. The retained [`reference.json`](reference.json) records the full
preorder tree after insertion, duplicate insertion, successful lookup, low and
high misses, failed and successful extraction, and deletion. Permanent Rust
tests assert the same topology after every operation. A deterministic 2,000-step
mixed-operation regression also checks lookup, mutation, extraction, deletion,
length, and sorted contents against `BTreeMap` after every step.

## Executable behavior and performance

[`compare_cmpcache_behavior.py`](compare_cmpcache_behavior.py) runs the pinned
Linux C reference and native Windows Rust executable on `LUSK6.lop`, whose
ordering path exercises the live comparison cache. With `--silent`,
[`comparison.json`](comparison.json) has exactly equal stdout, stderr, and exit
code without normalization: both report `Unsatisfiable` and exit zero.

[`benchmark_cmpcache.py`](benchmark_cmpcache.py) alternates five measured runs
of the pre-change Rust executable and the splay candidate on the same workload.
The retained [`benchmark.json`](benchmark.json) records exact output and exit
behavior in every measurement. Median wall time is 1.117 seconds for the
baseline and 1.175 seconds for the candidate, a 1.052 candidate/baseline ratio,
within the experiment's 1.10 comparability threshold. Ordinary verbose clause
progress was not used as the behavior gate because its address-sensitive work
order varies between independent allocator runs; silent theorem status is
stable, and the separate structural probe checks exact cache topology.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-112-quadtree-splay-topology\capture_c_topology.py `
  --output target\quadtree-splay-reference-check.json `
  --expected experiments\2026-07-18-112-quadtree-splay-topology\reference.json

cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-112-quadtree-splay-topology\compare_cmpcache_behavior.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\quadtree-cmpcache-comparison-check.json `
  --expected experiments\2026-07-18-112-quadtree-splay-topology\comparison.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-112-quadtree-splay-topology\benchmark_cmpcache.py `
  --baseline-exe target\quadtree-benchmark\baseline-eprover.exe `
  --candidate-exe target\default-reference\release\eprover.exe `
  --output target\quadtree-cmpcache-benchmark-check.json
```
