# Persistent SATCheck identity and selector lifecycle

## Status

This is the reviewed architecture for Bead `E_Rust_Port-9jt.4.9`. The
executable state-machine evidence is in
[`experiments/2026-07-30-012-persistent-satcheck-design/`](../experiments/2026-07-30-012-persistent-satcheck-design/).

The design is ready for a later opt-in production prototype. It is not a
production implementation, a performance claim, or permission to enable
periodic SATCheck. Experiment 017's trigger decision remains in force:
periodic SATCheck is default-off.

## Why append-only reuse is unsound

Each current SATCheck call:

1. chooses a grounding substitution from the current proof-state
   distribution;
2. imports five live clause sets;
3. encodes equality or predicate atoms into the main term bank;
4. densely renumbers those atoms for this call;
5. removes clauses carrying a currently pure literal;
6. gives every exported clause a call-local selector;
7. resets the SAT service, adds the guarded clauses, solves under all positive
   selectors, and resets again; and
8. maps failed selectors back through call-local exported-clause indices.

Experiment 017 observed substantial exact overlap, but most consecutive pairs
also contained deletions. Reusing only additions would leave deleted clauses
active and could turn a satisfiable current proof state into a false UNSAT
result. Reusing call-local integers would also conflate unrelated atoms or
selectors after renumbering.

## Required identities

### Atom identity

`Term::entry_no()` is a useful same-bank cache hint, not the durable logical
identity. Entry numbers are monotonic, but an encoded atom may be swept from
the term store and structurally re-created later with a different number.

The persistent boundary needs an owned `SatAtomKey` containing the complete
canonical structure:

```text
SatAtomKey {
    tag: variable | application,
    type_uid,
    function_or_variable_code,
    children: [SatAtomKey],
}
```

For equality atoms, build the key after the same instantiated-term ordering
and `Eqn::terms_tb_term_encode` normalization used today. The complete key,
not a digest alone, decides equality; a cached hash may accelerate lookup but
collisions must compare the full value. Function codes and interned type UIDs
are stable within one proof-state signature. A signature replacement changes
the session context and forces an epoch rebuild.

The first production prototype should keep an entry-number-to-`SatAtomKey`
memo only as a validated fast path. A miss or stale entry falls back to full
structural encoding.

### Source-clause identity

Use `ClauseDerivationRef` as `SatSourceId`. Its nonzero generation is the
preferred immutable process-local identity and survives clause-set movement
and visible proof renumbering. Generation-zero legacy references retain the
existing `(ident, CSSCPA source)` fallback.

At snapshot construction:

- reject two distinct current clauses with the same `SatSourceId`;
- retain the full encoded-clause fingerprint beside the source identity; and
- fall back to the existing fresh SATCheck path if source identity is
  ambiguous.

This fail-closed duplicate check is necessary before relying on legacy
generation-zero identities. A later broad change may guarantee nonzero
generations before every clause enters a proof-state set, but persistent
SATCheck does not require that migration as its first step.

### Encoded-clause identity

One source clause can ground differently as proof-state symbol frequencies
change. The persistent key is therefore:

```text
SatClauseKey {
    source: SatSourceId,
    canonical_literals: sorted unique [(polarity, SatAtomKey)],
}
```

Opposite polarities of the same atom make the clause tautological; the
snapshot omits it. Duplicate literals collapse. An empty canonical literal
list is a real empty clause.

The source identity provides proof provenance. The canonical literal list
provides revision identity. If the same source changes encoded content, its
old clause key retires and its new key receives a different selector.

## Selector invariant

Within one epoch, each `SatClauseKey` owns exactly one positive selector `s`.
The sole permanent representation is:

```text
C OR -s
```

For a current snapshot:

- assume `s` for every active clause key;
- omit the assumption for every retired key;
- never add `-s` as a permanent unit; and
- permit a cached key to reactivate by assuming its existing selector again.

Omission is sufficient. When a selector is not assumed, the SAT backend may
set it false and satisfy its guarded clause. A permanent negative unit would
prevent reactivation and would make epoch compaction mandatory for every
returning clause.

Current pure-literal behavior remains outside the persistent table:
recompute purity from the complete current imported snapshot exactly as
today, then reconcile the exported non-pure subset. A clause becoming pure
retires its selector for that call; one becoming non-pure inserts or
reactivates its key. Retired clauses never participate in current purity
analysis.

## Session state

A future wrapper can be shaped as:

```text
PersistentSatCheckSession<S> {
    service: S,
    state: Healthy | Poisoned,
    epoch,
    context,
    next_variable,
    atom_variables: SatAtomKey -> i32,
    clauses: SatClauseKey -> { selector, guarded_clause, source },
    active_by_source: SatSourceId -> SatClauseKey,
}
```

Atoms and selectors share one monotonically allocated positive `i32`
namespace inside an epoch. No solver call may observe a variable allocated to
both roles.

`context` contains every input whose change can invalidate identities without
changing clause content:

- proof-state/signature generation;
- grounding strategy;
- normalization flags;
- backend identity and relevant backend configuration; and
- encoding schema version.

A context change forces rebuild before solve.

## Reconciliation algorithm

1. Build the complete current source/atom/clause-key snapshot without
   mutating the backend.
2. Apply the current one-pass pure-literal export rule.
3. Detect retained, reactivated, new, and retired keys.
4. Estimate new atoms, selectors, permanent clauses, retired clauses, and
   variable pressure.
5. If no bound or context rule requests rebuild, stage all new mappings,
   append their guarded clauses, and publish the new active map only after
   every add succeeds.
6. Otherwise reset the service, increment the epoch, densely allocate all
   active atom variables followed by active selectors, add only active
   guarded clauses, and publish the rebuilt maps only after complete success.
7. Freeze a solve snapshot containing the epoch, ordered active assumptions,
   selector-to-source map, and encoded fingerprints.
8. Solve under the positive active selectors.

The proof state remains exclusively borrowed across encoding, solve, and core
mapping, so source clauses cannot change between the frozen snapshot and
terminal reconstruction.

## Database bounds

The experiment uses intentionally tiny thresholds to force transitions. A
production prototype must expose reviewed, finite limits and record them in
telemetry. The default design rule is:

```text
permanent_limit = max(P_min, P_factor * max(active, 1))
retired_limit   = max(R_min, R_factor * active)
```

Rebuild before a reconciliation that would exceed either limit. Rebuild also
when:

- the active snapshot is empty but the database is not;
- the next atom or selector would exceed the configured `i32` margin;
- the context changes;
- an explicit proof-state reset occurs; or
- the session is poisoned.

The first performance prototype should start conservatively with
`P_factor = 3`, `R_factor = 2`, and fixed minimum slack sized from measured
small calls. It must report active, retained, added, reactivated, retired,
permanent, atom, rebuild, and high-water counts. The constants are tuning
parameters, not semantic invariants.

After a successful rebuild:

```text
permanent clauses = active clauses
retired clauses   = 0
allocated atoms   = active distinct atoms
```

If the active snapshot itself cannot fit the variable or memory cap, return a
diagnostic and use no SAT result. Never truncate the snapshot.

## Failed-core and proof reconstruction

For an UNSAT outcome, validate all of the following before returning it:

1. the solve epoch still equals the session epoch;
2. every failed assumption is positive, unique, and a member of the frozen
   active assumption set;
3. every selector maps to exactly one active `SatClauseKey`;
4. every source resolves in the still-borrowed `ProofState`;
5. its current `ClauseDerivationRef` and encoded fingerprint equal the frozen
   values; and
6. the mapped active source clauses form an UNSAT selector core under the
   backend's existing failed-core validation.

Any failure discards the UNSAT result, poisons the session, and returns a
diagnostic. A retired selector in a failed core is a backend-contract failure,
not a source clause to resurrect.

Construct the terminal empty clause from the resolved current source clauses
in deterministic frozen-snapshot order. Retired source owners are unnecessary
for this reconstruction because retired selectors are not assumptions and
cannot be accepted in the failed core.

For checked CaDiCaL proofs, the proof scope contains:

- every permanent guarded clause, including retired ones; and
- one unit clause for each current positive selector assumption.

Inactive guarded clauses are harmless because their selectors remain free.
The independent checker validates the entire scope. The existing
`SatVerifiedProof` paths remain the evidence boundary.

## Failure and reset semantics

| Event | Session action | May solve? |
| --- | --- | --- |
| All staged adds succeed | Publish mappings and active set | Yes |
| Add fails after a partial append | Mark `Poisoned`; discard staged maps | No |
| Reset fails | Mark `Poisoned`; retain no trusted epoch transition | No |
| Rebuild add fails | Mark `Poisoned`; discard rebuilt maps | No |
| SAT | Keep database; assumptions expire | Yes, next reconciliation |
| Unknown / cancellation / decision limit | Keep database; assumptions expire | Yes, next reconciliation |
| Backend error or invalid core | Mark `Poisoned` | No |
| Explicit proof-state reset | Complete backend rebuild or clear session | Only after success |

Recovery from `Poisoned` is one complete reset-and-rebuild transaction. If it
fails, remain poisoned. The wrapper must never silently solve the previous
snapshot after reconciliation of a newer one failed.

The current `IncrementalSatService` trait owns a mutable service but not a
factory. Production recovery should either:

- require a reset that atomically replaces backend state, as the CaDiCaL
  implementation already does; or
- let the session own a backend factory so a failed reset can discard and
  recreate the service.

## Integration sequence

1. **Canonical snapshot only.** Add production `SatAtomKey`,
   `SatSourceId`, and `SatClauseKey` construction behind tests. Compare
   canonical snapshots with current DIMACS output; keep fresh solving.
2. **Shadow reconciliation.** Maintain the lifecycle table without reusing the
   solver. Emit overlap, retirement, growth, identity-ambiguity, and projected
   rebuild telemetry on held-out call streams.
3. **Opt-in persistent service.** Reuse the service only behind an explicit
   experimental option. On every sampled call, compare the persistent outcome
   and mapped core with a fresh service under equal inputs.
4. **Proof gate.** Independently check UNSAT traces whose permanent database
   includes retired clauses, then replay the reconstructed Umlaut proof.
5. **Performance gate.** Measure encoding, append, solve, rebuild, memory,
   common-work search cost, and solve complementarity. Stop if rebuild or
   structural-key cost consumes the observed overlap benefit.
6. **Policy review.** Only a separate preregistered held-out experiment may
   reconsider triggers or defaults.

## Required production tests

The experiment model covers the state-machine semantics. A Rust prototype
must additionally test:

- structural keys across term GC and re-insertion;
- equality direction normalization and typed atom separation;
- generation-zero duplicate-source rejection;
- one-pass purity transitions;
- source movement among all five imported clause sets;
- content replacement under a stable source reference;
- selector reactivation after retirement;
- `i32` boundary preflight;
- partial `add_clause` and `reset` failure injection;
- stale epoch, inactive selector, duplicate selector, and changed-fingerprint
  core rejection;
- checked proof scope with retired guarded clauses; and
- deterministic telemetry and proof-parent order.

No performance result can waive these correctness gates.

## Remaining risks

- Full structural atom keys may cost enough to erase some reuse benefit.
- CaDiCaL inprocessing and a deletion-heavy guarded database may retain more
  memory than clause-count bounds predict.
- Generation-zero clause identity needs production capture evidence; too many
  ambiguous cases may force frequent fresh fallback.
- The internal DPLL service clones its permanent database per solve, so
  persistence may help encoding but not its dominant solve cost.
- Experiment 017 found no useful trigger policy. Correct persistent state does
  not imply useful end-to-end proof search.

These are performance and integration questions for a later Bead, not gaps in
the selector-retirement semantics.
