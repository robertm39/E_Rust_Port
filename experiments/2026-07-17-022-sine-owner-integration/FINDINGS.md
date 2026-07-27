# SInE parser and owner integration audit

## Status

Completed for Bead `E_Rust_Port-j76.2.115`. Parser-created executable formula
ownership and destructive mixed clause/formula SInE selection are connected to
stable Rust owners. Raw-pointer owner discovery is not required for drop-in
behavior. The vendored C source remains unchanged.

## Executable ownership path

The executable parser uses its `FormulasForCnf` destination for every ordinary
input record. Parsed formula records retain their `WrappedFormula`; parsed CNF
clauses are wrapped with `WrappedFormula::form_clause_alloc`. The resulting set
is moved into `ProofState.f_axioms`, with stable formula entry ids and original
input metadata intact.

SInE runs before `FormulaSetCNF2`. Threshold, GSinE, and LambdaDef therefore
select the represented input owners rather than already-lowered approximations.
Only after pruning does CNF drain the surviving formula wrappers into clauses.
The `e_axfilter` path similarly keeps all `StructFofSpec` source sets alive and
prints borrowed selected objects, so it needs no destructive owner recovery.

## Movement and duplicate semantics

C stores each current `ClauseSet`/`FormulaSet` owner in the selected object's
intrusive links. Its stack movers unlink through that back-pointer and append to
the destination. A duplicate selected pointer consequently extracts the object
from the destination and reinserts it at the tail.

Rust proof-state movement uses unique clause identifiers and globally stable
formula entry ids. The movers receive the source and destination explicitly,
search the destination after the first move, and reinsert the same owned value.
Focused tests already pin duplicate relinking and final order for both kinds.
The represented proof-state GSinE test also selects a relation that crosses a
real `ClauseSet`/`FormulaSet` boundary and removes unrelated owners of both
kinds without cloning.

The generic C-shaped stack helpers retain that literal move-by-id behavior for
small staged callers. The production proof-state replacement avoids quadratic
repeated deque/set scans: it computes each id's last selected position, drains
each source owner once into an ordered map, and rebuilds the selected sets in
last-occurrence order. A 2,048-formula regression reverses the entire selection
and repeats its first selected id, validating both scale and tail relinking.

## End-to-end regression

The new executable regression parses one CNF goal, one related FOF axiom, and
unrelated CNF and FOF axioms. Prune-only GSinE follows the shared symbol from
the CNF record to the FOF record, retains their original `cnf(...)` and
`fof(...)` wrapper shapes and source metadata, removes both unrelated records,
and exits before CNF. This joins the previously separate parser-owner and mixed
selection evidence.

Stable ids plus phase-owned sets are the intentional safe replacement for C's
intrusive owner discovery. Hash-backed bulk replacement is expected `O(n + k)`
for `n` owned and `k` selected objects and moves each object at most once; its
observable order comes only from the selection vector, not hash iteration.
SInE relation traversal remains unchanged.

## Validation

- focused parser-owner, mixed proof-state, duplicate movement, 2,048-owner bulk
  movement, and new mixed executable SInE tests pass;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
