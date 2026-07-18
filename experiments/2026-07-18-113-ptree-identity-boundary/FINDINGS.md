# PTree identity boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.25`. The generic Rust `PTree` preserves
the exact C top-down splay topology and now exposes C-shaped mutating `find`
semantics separately from non-reorganizing binary lookup. Its two direct
production owners have an explicit live-allocation identity boundary. The
vendored C checkout remains unchanged.

## API and topology

C `PTreeFind` splays on both successful and failed lookup, whereas
`PTreeFindBinary` performs a read-only binary walk. Rust previously kept the
exact implementation behind `find_splayed` but made the shorter `find` name a
read-only lookup. This slice aligns the primary methods with C: `find` and the
compatibility alias `find_splayed` splay, while `find_binary` alone leaves the
topology untouched.

[`capture_c_topology.py`](capture_c_topology.py) compiles the isolated
[`probe_ptrees.c`](probe_ptrees.c) against the pinned unchanged `BASICS.a`.
The retained [`reference.json`](reference.json) records every node and null
child after insertion, duplicate insertion, mutating lookup, binary lookup,
low/high misses, failed/successful extraction, and deletion. A permanent Rust
test asserts the same complete shapes. Focused tests also cover C-order merge,
copy, intersection, root extraction, root-right-left stack traversal, in-order
visiting, and debug output.

## Production owners and identity policy

[`audit_ptree_owners.py`](audit_ptree_owners.py) finds exactly two direct generic
`PTree` instantiations outside the implementation:

- typed clause formula-closure rendering in `clauses/clause.rs`; and
- term-formula free-variable collection in `clauses/clausefunc.rs`.

Both key the tree with `term_identity_id`, which is the live address returned by
`Rc::as_ptr`, and both consume C's root-right-left `PTreeToPStack` order. This
matches C's native `uintptr_t` pointer comparison policy within each process.
Numeric identities and their relative order are not promised to match across C
and Rust allocators. New ownership-oriented APIs should use stable typed handles
unless the caller deliberately needs this compatibility traversal.

The allocator dependence is already observable evidence, not a hypothetical:
the earlier `sledgehammer.p` trace found same-sort binder permutations that no
semantic variable field explains. It remains correct to report that textual
difference instead of adding a problem-specific ordering rule. Other C
pointer-tree roles use owner-specific safe representations when their observable
contract is set membership, stable IDs, or an explicitly reconciled traversal.

## Exact owner comparison

[`compare_ptree_owner.py`](compare_ptree_owner.py) sends the retained
[`inputs/typed-variables.p`](inputs/typed-variables.p) fixture to both executables
through stdin, avoiding platform path normalization. The proof contains a typed
`tcf` clause whose three-variable closure uses the direct PTree owner.
[`comparison.json`](comparison.json) is byte-exact across the pinned Linux C
reference and native Windows Rust executable: stdout, stderr, theorem status,
typed proof, and exit code all match.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-113-ptree-identity-boundary\audit_ptree_owners.py `
  --output target\ptree-owner-audit-check.json `
  --expected experiments\2026-07-18-113-ptree-identity-boundary\owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-113-ptree-identity-boundary\capture_c_topology.py `
  --output target\ptree-topology-reference-check.json `
  --expected experiments\2026-07-18-113-ptree-identity-boundary\reference.json

cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-113-ptree-identity-boundary\compare_ptree_owner.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\ptree-owner-comparison-check.json `
  --expected experiments\2026-07-18-113-ptree-identity-boundary\comparison.json
```
