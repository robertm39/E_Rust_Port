# Cleanup maintenance order

## Status

Completed for Bead `E_Rust_Port-j76.2.51`. The Rust cleanup pipeline now
measures proof-state storage at C's position after orphan cleanup and special
forward contraction/reweighting. The vendored C checkout remained unchanged.

## Compatibility gap

C calls `ProofStateStorage` only after the first two mutating maintenance
gates. The Rust default wrapper previously computed its estimate before
entering the shared helper. If eager orphan deletion or special forward
contraction lowered storage below `delete_bad_limit`, Rust could still enter
delete-bad using the stale higher estimate, discard valid clauses, and mark an
otherwise complete search incomplete.

The same wrapper constructed one parent-liveness snapshot before all cleanup.
The corrected default path now builds a fresh generation-qualified snapshot
inside each orphan-deletion gate that actually fires. The injected low-level
test/alternate-owner API remains available and supplies its explicit storage
value and parent predicate through the same ordered core.

## Evidence

[`audit_cleanup_order.py`](audit_cleanup_order.py) pins ten C/Rust contracts:
the three gate order, C's late storage call, Rust's late estimator callback,
fresh liveness at both possible orphan gates, saturation placement after
clause processing, C-shaped forward-maintenance output, and the permanent
post-orphan storage regression. All 10/10 contracts pass in
[`audit-reference.json`](audit-reference.json).

The regression sets `delete_bad_limit` one unit below pre-cleanup storage, then
eagerly removes an orphan whose dead parent is archived. Post-cleanup storage
is below the limit, so C does not enter delete-bad. The corrected Rust path
matches that decision; the old early snapshot would have entered the branch
and failed because the fixture intentionally has no active HCB.

[`compare_cleanup_output.py`](compare_cleanup_output.py) runs unchanged C
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and optimized Rust on the
three-clause fixture with `--forward-contract-limit=0`. Exit status, complete
stdout, and complete stderr are byte-exact. The six extracted maintenance
lines are also exact and have SHA-256
`97FEE60C75F77578B425C993136347F8A68E82202FF112C7F310C9D49CC43333`;
the retained result is [`output-reference.json`](output-reference.json).

## Arena decision

The earlier stable-identity audit already proves that `ClauseDerivationRef`
replaces C's raw parent-pointer identity across selection and periodic cleanup,
including reused visible clause identifiers. A proof-wide maintained clause
arena would add mutation bookkeeping and per-clause memory to avoid occasional
bulk snapshots. That is an optional post-compatibility optimization to pursue
only after representative profiling, not a remaining correctness or lifetime
gap in this cleanup port.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-086-cleanup-maintenance-order\audit_cleanup_order.py `
  --repo . `
  --expected experiments\2026-07-17-086-cleanup-maintenance-order\audit-reference.json

cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-086-cleanup-maintenance-order\compare_cleanup_output.py `
  --c-eprover /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --rust-eprover target\release\eprover.exe `
  --output target\cleanup-maintenance-output.json `
  --expected experiments\2026-07-17-086-cleanup-maintenance-order\output-reference.json
```

## Validation

- focused cleanup regressions passed;
- source/order audit: 10/10 contracts passed;
- live C/Rust comparison: 1/1 process exact, including six maintenance lines;
- all-target/all-feature suite: 4,299 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy and formatting passed;
- all four C-source documentation integrity gates passed;
- optimized all-feature `eprover` build and experiment script compilation
  passed; and
- vendored C worktree remained clean.
