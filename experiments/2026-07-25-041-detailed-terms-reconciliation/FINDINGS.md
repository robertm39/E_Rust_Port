# Detailed TERMS reconciliation

## Status

Accepted for the 29 remaining open `TERMS` records under Beads
`E_Rust_Port-j76.4`. Direct source review found one enforceable ownership gap:
Rust's intrusive `TermTree` was publicly cloneable even though cloned roots
share and mutate the same term-cell links. The owner is now crate-private and
non-cloneable, confining production construction to `TermCellStore` without a
runtime field, layout change, or hot-path branch. The other 28 records are
resolved compatibility, ownership, parser, output, API, or measured
performance decisions. The vendored C checkout remains unchanged.

## Review decisions

| Records | Decision |
|---|---|
| 1220, 1224, 1237, 1259, 1303, 1305, 1324 | Preserve the documented C boundary: AC early rejection, fixpoint-oracle fragment classification, monomorphic imitation limit, original-term HO pattern fallback, typed application preconditions, lambda-body order traversal, and C-width unchecked variable counts. These are observable or assertion-level compatibility contracts, not missing Rust entry points. |
| 1218, 1231, 1235, 1246, 1279, 1293, 1315 | Retain Rust's deterministic or safe ownership replacement: duplicate AC arguments without pointer-order dependence, live signature query borrows, typed GC handles, explicit WHNF binding vectors, safe let-scope vectors/restoration, immutable child borrows, and the measured compact shared-term representation. |
| 1311 | Close the real safe-API hole. `TermTree` is now crate-private and non-cloneable, so external safe code cannot duplicate or independently assemble intrusive roots; the sole production owner is the hash-bucket vector inside `TermCellStore`. |
| 1280, 1281, 1282, 1289, 1290, 1294, 1316, 1321, 1322 | Keep the represented mixed term/formula grammar and output boundaries explicit. The production formula owner, checked/simple term parsers, typed `$ite` recovery, scoped quantifier allocation, list-aware per-signature printing, entry-number ordering, and UID-ordered type declarations have landed. Remaining FOOL recovery limits and allocator-sensitive presentation permutations are declared grammar/presentation boundaries rather than absent low-level APIs. |
| 1256, 1263, 1306 | Retain the current API and scratch-storage shapes. The complete-MGU boolean wrapper intentionally hides the internal prefix result, the match stack's measured four-pair inline Rust policy already outperformed the alternatives, and one variable-collection `Vec` remains unoptimized absent a measured end-to-end win. |
| 1254 | No beta-only repair is missing. Direct comparison shows that neither C nor Rust calls `flatten_and_make_shared` inside `do_beta_normalize_db`; both share changed rebuilt tops directly. The repair belongs to the recursive eta paths in both implementations. |
| 1317 | The note is stale. `ProofState` now owns `fresh_vars`, proof-control inference passes it to equality resolution, factoring, and indexed paramodulation, and each family preserves its C reset/consume policy. Standalone helpers deliberately retain isolated scratch banks. |

## Audit

[`audit_terms_reconciliation.py`](audit_terms_reconciliation.py) pins the
exact 29 migrated identities and content hashes, verifies that each remains a
`terms` record, and checks ten grouped C/Rust ownership, parser, output,
higher-order, inference, determinism, performance, and compatibility
contracts. Its source digest covers the 16 implementation files plus the
port-status ledger used in the review. The audit is independent of issue
status, so it remains reproducible after closure.

The static `TermTree` check is the regression for the implementation change:
it requires crate-private visibility, rejects the clone derive, and pins the
ownership rationale next to the type. Existing term-store insertion, lookup,
extraction, splay-topology, GC, full prover, and support-tool tests exercise
the unchanged runtime behavior.

## Validation

On ephemeral Ubuntu 24.04 worker `e-rust-codex-260726-191044-7524` with Rust
1.97.1, exact source snapshot
`20e0c9fed1f0e1af00f1b126c5a4a82ab9c384455f245a4e1bac31ba69b85e70`
passes:

- `cargo fmt --all -- --check`;
- strict all-target/all-feature pedantic Clippy;
- all 4,416 library and 11 integration tests, 4,427 total;
- the native optimized build of every Rust binary;
- compile-only Windows GNU x64 all-target/all-feature tests and every release
  binary; and
- PE32+ inspection of `eprover.exe`.

The exact native release `eprover` SHA-256 is
`eda30ace57d6bc691216ab5fb6422df42099e92432e573d503cf61063675dada`;
the compile-only Windows GNU x64 executable SHA-256 is
`6e71f97b224750b954193c9ba422f1ace2fdda1f81893435e6393bd06c674771`.
Against unchanged cached FOL/HO C reference binaries, the optimized Rust
binaries pass all 50 main-prover cases with zero unexpected differences and
one declared presentation difference, plus all 216 support-tool cases with
zero unexpected differences and 15 declared differences.

The 29-record source audit and documentation coverage, `Change Later` wording,
local-link, and manual-regeneration validators pass. The local C checkout is
clean. [`validation-reference.json`](validation-reference.json) pins the
snapshot, binary, matrix-report, cross-compile, and test counts. No Rust or C
toolchain ran on the local Windows host.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-041-detailed-terms-reconciliation/audit_terms_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-041-detailed-terms-reconciliation/audit-reference.json
```
