# Persistent SATCheck identity design preregistration

## Scope

This experiment addresses Bead `E_Rust_Port-9jt.4.9`. It is a
production-neutral architecture and state-machine study. It must not retain a
SAT backend across production calls, change a SATCheck trigger, or enable
periodic SATCheck by default.

Experiment 017 found 68.2% median exact-clause retention across 126
consecutive captured pairs, but only 41 pairs (32.5%) were add-only. Current
calls rebuild a `SatClauseSet`, locally renumber atoms, allocate selectors
above the call-local maximum atom, reset the selected service before solving,
and reset it again afterward. A deletion-aware identity and selector lifecycle
is therefore required before performance measurement is meaningful.

## Audited production boundaries

The current path establishes these constraints:

- `TermBank` entry numbers are monotonic during one bank lifetime, but an
  encoded atom can be garbage-collected and structurally re-created with a new
  entry number. Entry number alone is not a durable logical atom key.
- `ClauseDerivationRef` gives a stable process-local source identity when its
  generation is nonzero, surviving set movement and visible proof-output
  renumbering. Encoded content can nevertheless change under a different
  grounding choice or an in-place clause revision, so source identity alone is
  not an encoded-clause identity.
- CaDiCaL and the internal service retain permanent clauses and expire
  assumptions after each solve. Both validate failed-assumption cores. A
  selector-guarded clause can therefore be disabled by omitting its positive
  selector assumption; adding a permanent negative selector unit would make
  later reactivation impossible.
- A SATCheck UNSAT result is terminal. Its failed assumptions must map only to
  currently active source clauses before the empty clause is reconstructed.
- A checked CaDiCaL proof is scoped over every permanent guarded clause plus
  the current assumption units. Retired guarded clauses remain sound in that
  scope because their unassumed selectors can disable them.

## Frozen candidate design

The experiment model will implement the following identities and lifecycle:

1. `AtomKey` is a complete canonical structural atom value, not a hash or
   call-local integer. Production integration would encode normalized term
   tags, function codes, sort identities, and child structure and compare the
   complete value on hash collisions.
2. `SourceId` represents the immutable-generation identity supplied by
   `ClauseDerivationRef`.
3. `ClauseKey` is `(SourceId, canonical signed AtomKey multiset)`. Replacing a
   source's grounded content retires the old key and inserts or reactivates the
   new key.
4. Every clause key owns one positive selector inside an epoch. The permanent
   solver clause is `C OR -selector`. A solve assumes exactly the selectors of
   the current exported snapshot. Retirement means omission from assumptions,
   never a permanent unit.
5. The solve snapshot freezes `epoch`, selector-to-source mapping, and clause
   fingerprints. Every failed assumption must be positive, unique, active,
   and resolvable to unchanged current source content. Anything else fails
   closed and poisons the session.
6. Rebuild resets the backend, increments the epoch, drops retired
   clause/atom mappings, densely renumbers active atoms and selectors, and
   re-adds only active guarded clauses.
7. A partial add/reset/backend failure poisons the session. No solve is
   permitted until a complete rebuild succeeds. SAT or Unknown leaves the
   permanent database valid because assumptions expire.

The model's deliberately small test thresholds are:

- rebuild when projected permanent clauses exceed
  `max(8, 3 * max(active clauses, 1))`;
- rebuild when projected retired clauses exceed
  `max(4, 2 * active clauses)`; and
- rebuild before the next variable would exceed the configured variable cap.

These constants force lifecycle coverage; they are not proposed production
tuning. A production wrapper must expose larger bounded limits and profile
them before integration.

## Frozen falsification matrix

The model is acceptable only if all of these checks pass:

1. additions preserve existing selector and atom assignments inside an epoch;
2. deletion disables a clause without a permanent negative selector and can
   reactivate the same clause key safely;
3. same-source content replacement retires the old selector;
4. fresh and persistent SAT/UNSAT outcomes agree after every step;
5. every reported failed-selector core contains only active sources and the
   mapped source clauses are independently UNSAT;
6. randomized insertion order and forced rebuild renumbering do not change
   outcomes or source-core identities;
7. repeated replacement/deletion respects the frozen permanent/retired
   database bounds after reconciliation;
8. injected partial-add and reset failures prohibit solving and recover only
   through a successful complete rebuild;
9. explicit context reset changes the epoch and leaves no retired database
   state; and
10. empty, duplicate-literal, tautological, and contradictory-unit snapshots
    agree with the fresh oracle.

At least 100 deterministic randomized transition traces with at least 50
steps each must pass in addition to focused examples. The oracle enumerates
all assignments over a bounded atom set and is independent of the persistent
guarded-clause evaluation.

## Decision

- `design-ready`: every invariant is explicit and the full falsification
  matrix passes on Windows and the retained Ubuntu runner.
- `revise`: any semantic, core-mapping, growth-bound, or recovery test fails.

Even `design-ready` authorizes only a later production prototype and
performance gate. It does not authorize current SATCheck integration or a
default-policy change.
