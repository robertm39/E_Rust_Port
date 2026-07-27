# `ccl_clausefunc` ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.109`. The orphan-deletion and
injectivity-recognition items in the migrated statement were mostly stale:
their production proof-state integrations and derivation metadata were already
present. The reconciliation found and fixed one exact-owner defect in
injectivity preprocessing. The vendored C source remains unchanged.

## Orphan ownership mapping

C `ClauseIsOrphaned` checks only the first generating derivation operation, its
direct clause parents, and immediately following `DCCnfAddArg` parents. Rust's
`clause_is_orphaned_with` preserves that scan boundary. C tests `CPIsDead`
through stable raw parent pointers; Rust derivations instead store compact
references containing visible id, source, and an opaque generation.

`ProofState::clause_parent_is_dead` resolves those exact references over every
live and archive owner. Production proof-control cleanup and selection use the
same identity through a source-aware liveness snapshot, so equal visible ids do
not alias. `clause_set_delete_orphans_with` also preserves C's two-phase
mark/clear followed by marked deletion. Its injected liveness predicate is an
ownership boundary for the low-level clause helper, not a missing proof-state
integration.

## Injectivity mapping and fix

Rust already preserves C's two-literal recognition gates, typed inverse-Skolem
construction, `CPIsPureInjectivity`, source TPTP/SOS metadata, proof depth and
size increments, and `DCInvRec` parent. Higher-order generation calls the
recognizer behind `inverse_recognition` and inserts the result into `tmp_store`.
Preprocessing calls `clause_set_replace_injectivity_defs` behind
`replace_inj_defs`. C contains a TODO for a separate proof-documentation call,
so the Rust path is not missing an observable C proof event.

C defers iteration with an exact intrusive-list pointer. Rust had approximated
that ownership by snapshotting visible clause ids and later calling
`find_by_id`/`extract_by_id`. A non-injectivity clause followed by an
injectivity definition with the same visible id caused both visits to resolve
the first clause, silently skipping the definition. The replacement path now
snapshots `ClauseDerivationRef` values and performs exact lookup and extraction.
`ClauseSet::extract_by_derivation_ref` uses the normal set extraction path, so
indexes, counters, and ownership bookkeeping remain centralized.

The regression constructs two owners with the same visible id and distinct
generations, with the non-definition first. It requires the non-definition to
remain active, the exact definition to move to the archive, and the generated
pure-injectivity clause to enter the active set.

## Performance

The corrected lookup is a linear sparse-slot scan, matching the previous
`find_by_id` and `extract_by_id` complexity. It adds no search-loop data
structure or asymptotic cost. A future stable owner arena could make exact
handle lookup constant-time, but that post-compatibility redesign is not needed
for correctness here.

## External comparison status

This reconciliation is supported by direct C/Rust source mapping and Rust
regressions. The checked C executable is a Linux binary, but the active Windows
account has no installed WSL distribution; no unobserved executable comparison
is claimed.

## Validation

- the duplicate-visible-id exact-owner regression passes;
- the existing first-definition/duplicate-definition compatibility regression
  passes;
- stable derivation references continue to survive sparse-store compaction and
  set transfer;
- all focused injectivity, orphan-deletion, production parent-liveness, and
  higher-order inverse-generation tests pass;
- all 4,230 default library tests and all 4,235 all-feature library tests pass,
  together with every binary and integration target;
- strict all-target/all-feature Clippy passes with warnings denied; and
- formatting, source-document coverage, Change Later wording, links, and
  regeneration preservation all pass.
