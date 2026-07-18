# AC-Axiom Scan Ownership

## Status

Completed for Bead `E_Rust_Port-j76.2.57`. The `che_axiomscan` detectors,
signature mutations, compact proof-parent ownership, and activation return
value are represented. The vendored C checkout remained unchanged.

## Compact Parent Ownership

C appends a raw `Clause_p` to `sig->ac_axioms` only when an associative or
commutative property first transitions from unset to set. Rust mirrors this
with `ClauseDerivationRef`: the visible clause identifier and a nonzero
generation preserve distinct proof parents after clauses move between owners.

The permanent set-scan regression starts with fresh associative and
commutative clauses that deliberately share visible ID `51`. It verifies that
both property transitions are installed, both generation-qualified parents
are recorded in clause-set scan order, and a second scan records no duplicate
parents. This closes a blind spot in the older test, which rescanned a cloned
set only after the commutative property was already installed.

## Return-Value Compatibility

`ClauseScanAC` and `ClauseSetScanAC` return C's activation signal rather than a
generic "signature changed" result. An associativity-only scan mutates the
signature and returns false; any detected commutativity axiom returns true,
even when that property was already present. Proof-state initialization uses
that boolean to decide whether AC handling is active.

The three executable fixtures retain the observable boundary:

- `associative.p` prints `f is associative` without the activation line.
- `commutative.p` prints `f is commutative` and activates AC handling.
- `ac.p` prints `f is AC` and activates AC handling.

[`compare_axiomscan.py`](compare_axiomscan.py) is exact between unchanged C at
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and Rust for those scan lines,
four statistics per fixture, exit code, SZS status, and stderr.
[`reference.json`](reference.json) has SHA-256
`49A8A112A70744F3E27273F83AB9D4B8BB01ED2A50275409D3DD3F54F640A244`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-080-axiomscan-ac-ownership\compare_axiomscan.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\axiomscan-reference.json `
  --expected experiments\2026-07-17-080-axiomscan-ac-ownership\reference.json
```

## Compatibility Decision

The single C boolean remains intentionally asymmetric because proof-control
callers treat it as commutativity/AC activation, not as a general mutation
indicator. A future cleaner API may return both facts, but the compatibility
surface must retain the current activation decision.
