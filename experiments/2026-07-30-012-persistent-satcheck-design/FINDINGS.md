# Persistent SATCheck identity design findings

## Decision

`design-ready`.

The audited architecture and experiment-only model satisfy the Bead's identity,
selector lifecycle, deletion/retirement, growth-bound, reset/failure, and
proof-reconstruction criteria. Production SATCheck remains unchanged and
default-off. A later opt-in Rust prototype must pass the integration sequence
in
[`docs/persistent-satcheck-design.md`](../../docs/persistent-satcheck-design.md)
before any performance gate.

## Production audit

The current implementation confirms the premise of Experiment 017:

- `SatClauseSet` owns a call-local `BTreeMap<i64, i32>` from encoded term entry
  number to dense SAT atom;
- the selected incremental-service path guards every exported clause with a
  selector above that call's maximum atom;
- `with_fresh_incremental_service` resets before and after every solve;
- failed selectors map through their position in the call-local selector and
  exported-index vectors;
- `ProofState::sat_import_context` snapshots four processed sets plus
  `unprocessed`;
- `TermBank` entry numbers rise monotonically but swept structural terms may
  later be reinserted under a new number; and
- `ClauseDerivationRef` already supplies generation-backed process-local proof
  identity with a legacy `(ident, source)` fallback.

Therefore neither the current atom integer, selector integer, exported index,
nor source identifier alone is enough. The accepted design uses a complete
structural atom key and `(source identity, canonical encoded content)` clause
key.

## Validated lifecycle

The experiment model implements:

- complete structural atom values, including sort and child structure;
- canonical duplicate removal and tautology detection;
- a stable source generation plus encoded-content clause key;
- one permanent `C OR -selector` clause per key and positive assumptions only
  for the current snapshot;
- deletion by assumption omission and safe selector reactivation;
- source-content replacement through a new selector;
- a shared collision-free atom/selector variable namespace;
- epoch rebuild and dense renumbering under context, growth, empty-state, or
  variable pressure;
- explicit permanent and retired database bounds;
- transactional publication after clause additions;
- poisoning after partial add, reset, rebuild, capacity, or core-mapping
  failure; and
- failed-selector minimization and independent source-clause UNSAT checking.

The model does not pretend that selector omission deletes backend clauses.
Retired guarded clauses remain permanent until a bounded rebuild.

## Falsification results

The exact same model and test sources passed on Windows 11 / Python 3.14.3 and
Ubuntu 24.04 / Python 3.12.3.

The focused cases cover:

1. add, delete, reactivate, and same-source replacement;
2. core mapping after unrelated contradictory clauses retire;
3. forced atom renumbering with stable source core;
4. repeated replacement under permanent/retired growth bounds;
5. partial incremental-add failure and recovery;
6. reset failure and recovery;
7. partial rebuild and active-capacity failure;
8. explicit context reset;
9. empty, duplicate, tautological, and contradictory-unit snapshots; and
10. typed and nested structural atom separation.

The frozen randomized campaign used seeds `0..99` and 60 transitions per
seed:

| Metric | Result |
| --- | ---: |
| Traces | 100 |
| Transitions | 6,000 |
| Fresh-oracle outcome checks | 6,000 |
| Incremental transitions | 5,058 |
| Epoch rebuilds | 942 |
| SAT snapshots | 1,111 |
| UNSAT snapshots | 4,889 |
| Independently checked mapped source cores | 4,889 |
| Maximum permanent clauses | 14 |
| Maximum retired clauses | 9 |

Every persistent SAT/UNSAT outcome matched an independent truth-table solve of
the current logical snapshot. Every mapped core contained only active sources
and was independently UNSAT. Every successful reconciliation satisfied its
variable-namespace and database-growth invariants.

Five existing all-feature Rust boundary tests also passed on Ubuntu with the
pinned CaDiCaL 3.0.1 source: the two SATCheck reset/selector-core tests,
CaDiCaL permanent-clause/ephemeral-assumption behavior, internal-service
permanent-clause/ephemeral-assumption behavior, and CaDiCaL model/failed-core
validation.

Ubuntu `campaign-result.json` has result id
`3e49c19f780fad8b67e34627eb812c1d64f83442d2240af61015fc2ff3937f98`
and file SHA-256
`4ffa015efa3ffee2a88f6e0a7413aad69179c767719d19816028477f912a578e`.
The measured source hashes recorded inside it are:

- model:
  `c982ac5efcb78b2bb4c650f6454124e47e271fde51ea57a697809313ceff835f`;
- tests:
  `ab841aad18ac99ad0cb054c52c3cc83e1660de6482741ff6351f7835c6f3254a`;
  and
- preregistration:
  `7dea509c42b9f1acb3df5d9181fd11558399adc94bfe452a480adc83e9e13e15`.

The initial forced-renumber test did not actually exceed its configured
variable cap. Tightening it then exposed a second test ambiguity: two
independent contradictions allowed a deletion-minimized core to choose either
pair. The final frozen-intent scenario forces a real rebuild, changes the atom
number, and preserves the only contradictory source pair. These were
test-scenario corrections, not model changes or observed-result selection.

## Proof rule

A production solve must freeze the epoch, active selectors, source mapping,
and content fingerprints. An UNSAT result is acceptable only if every failed
assumption is a unique positive active selector and every mapped current
source still has the frozen identity and encoded content. A retired selector
is a backend-contract failure.

The checked CaDiCaL scope may safely include retired guarded clauses: with no
positive selector unit they can be disabled. The scope must also include the
current positive selector units, as the existing CaDiCaL proof path already
does for assumptions.

## Limits

This model proves the proposed lifecycle's bounded propositional semantics. It
does not validate a concrete Rust structural-term encoder, term-GC behavior,
generation-zero prevalence, CaDiCaL memory under long guarded histories, or
end-to-end speed. It uses deliberately small bounds to force rebuild coverage;
they are not production tuning.

The next justified work is the staged canonical-snapshot and shadow-lifecycle
prototype in the design. Direct live backend reuse or trigger changes would
skip required correctness evidence.
