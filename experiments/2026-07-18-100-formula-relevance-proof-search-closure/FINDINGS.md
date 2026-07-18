# Formula Relevance Proof-search Closure

## Status

Completed for Bead `E_Rust_Port-j76.2.38`. The migrated gap was already
implemented and covered by the later formula-relevance ownership and ordering
slices. A fresh proof-search comparison closes the remaining wording-specific
surface. The vendored C checkout remained unchanged.

## Owner and ordering decision

Unchanged C runs `ProofStateSinE`, then `ProofStateRelevancyProcess`, then
`FormulaSetCNF2`. Relevance pruning constructs new clause and formula axiom
sets, moves the requested relevance levels into them, replaces both proof-state
owners, and reports the combined cardinality delta. The main driver adds that
delta to the SInE removal count used by proof statistics.

Rust preserves the same transition. `apply_relevance_pruning` passes both live
owners to `clause_formula_sets_relevance_prune`, replaces both proof-state sets,
and returns the combined removed count before formula CNF. The ordinary
proof-search path adds it to the SInE count and later renders the same combined
statistics field.

[`audit_relevance_owners.py`](audit_relevance_owners.py) passes 14/14 source,
owner, ordering, permanent-regression, and retained-reference checks. The
earlier owner matrix remains 2/2 matching, and the earlier ordering matrix
remains 3/3 matching across five fresh C processes per case.

## Exact proof-search comparison

[`compare_proof_search.py`](compare_proof_search.py) runs isolated unchanged C
at commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release on a
mixed owner fixture containing a formula conjecture, relevant and unrelated
formula axioms, and relevant and unrelated clause axioms.

Both executions are exact for:

- `% Clause set closed under restricted calculus!` and `SZS status GaveUp`;
- exit code `10` and empty stderr;
- five parsed axioms and two relevance/SInE removals;
- three initial and initial-saturation clauses;
- three processed clauses, three current processed clauses, and no current
  unprocessed clauses; and
- zero live archived formulas after formula CNF drains the active formula
  owner before proof-control statistics.

The retained [`reference.json`](reference.json) has SHA-256
`44250689FBEA56FAEABE406211027045098B8C4FC9E291CF583E081A5CFBF9C8`.
The compared Rust `eprover.exe` has SHA-256
`E4CAB1204C7F57AA50BDA1CD71FE869FFB4FC466A01506311A95C51E7F488A69`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-100-formula-relevance-proof-search-closure\audit_relevance_owners.py `
  --output target\formula-relevance-owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-100-formula-relevance-proof-search-closure\compare_proof_search.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\formula-relevance-proof-search.json `
  --expected experiments\2026-07-18-100-formula-relevance-proof-search-closure\reference.json
```

## Compatibility decision

The issue requires no additional owner or preprocessing API. Formula owners
already participate destructively in relevance pruning before CNF, and both the
retained pre-CNF owner view and ordinary proof-search accounting match C.
