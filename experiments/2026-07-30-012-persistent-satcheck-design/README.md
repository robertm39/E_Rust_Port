# Persistent SATCheck identity design

This experiment addresses Bead `E_Rust_Port-9jt.4.9`, following the
incremental-reuse evidence in
[`2026-07-29-017-ground-sat-trigger-evaluation`](../2026-07-29-017-ground-sat-trigger-evaluation/).

It is architecture work only. An experiment-local state-machine model checks
stable atom/source identity, selector activation and retirement, bounded
rebuilds, fail-closed recovery, and UNSAT-core source mapping against a fresh
brute-force oracle. It does not change production SATCheck behavior or enable
periodic checking.

The accepted architecture is
[`docs/persistent-satcheck-design.md`](../../docs/persistent-satcheck-design.md).
`FINDINGS.md` records the cross-platform falsification result and
`COMMANDS.md` gives the reproduction commands.
