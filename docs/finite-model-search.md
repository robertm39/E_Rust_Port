# Typed Finite-Model Search

Umlaut has an explicit, nondefault bounded finite-model worker:

```text
umlaut --finite-model-search problem.p
```

The worker is not part of `--auto`, `--auto-schedule`, or any built-in
portfolio. It runs after formula clausification and before ordinary clausal
preprocessing, so it sees typed first-order CNF without transformations that
could obscure a complete interpretation. The path requires a build with the
optional `cadical-static` feature. A default package build accepts the option
but fails closed with `SZS status Inappropriate`; it never silently substitutes
an unsuitable backend or claims a model.

## Supported fragment and output

The worker supports many-sorted first-order clauses over `$i` and declared
uninterpreted native sorts. It supports equality, predicates of every
first-order arity, constants, positive-arity functions, and nested function
terms. It deliberately rejects higher-order terms, Boolean terms used as
ordinary data, arithmetic sorts and interpreted symbols, distinct objects,
special symbols, missing types, and inconsistent symbol types.

A success is printed only after the decoded SAT assignment has been
independently evaluated against every ground instance at the selected domain
sizes. Output contains:

- `SZS status Satisfiable`, or `CounterSatisfiable` when the clausified input
  contains a conjecture;
- an `SZS output start FiniteModel` / `end FiniteModel` section;
- typed names for every element of every native sort;
- domain closure and pairwise-distinctness axioms;
- exactly one equation for every active row of every function table; and
- one positive or negative atom for every active predicate-table row.

This is a positive-only trust boundary: a malformed, incomplete, out-of-domain,
or semantically invalid SAT model becomes `SZS status Error`, never a
satisfiability claim.

## Bounds and resource controls

Domain vectors are deterministic: increasing total cardinality, then
lexicographic sort cardinality. One incremental SAT database is extended as
new vectors become reachable. All resource controls are hard fail-closed
limits:

| Option | Default | Meaning |
| --- | ---: | --- |
| `--finite-model-max-size=N` | 3 | Maximum cardinality of each sort |
| `--finite-model-max-vectors=N` | 2048 | Maximum domain vectors examined |
| `--finite-model-max-ground-instances=N` | 5000000 | Maximum distinct clause groundings |
| `--finite-model-max-clauses=N` | 10000000 | Maximum permanent SAT clauses |
| `--finite-model-max-variables=N` | 10000000 | Maximum SAT variables |
| `--finite-model-sat-timeout=N` | 5 | Per-vector SAT deadline in seconds |

Every value must be a positive integer. Process CPU/memory limits and caught
termination signals remain active. SAT deadline/cancellation, a configured
encoding limit, or an external resource stop produces `SZS status
ResourceOut`. Exhausting all configured domain vectors with checked UNSAT
answers produces `SZS status GaveUp`. Unsupported syntax produces
`Inappropriate`; solver protocol, backend, decoding, or semantic-check
failures produce `Error`.

Each attempted vector emits one stable `% FNT bound` comment with the native
sort sizes, new and cumulative grounding counts, new and cumulative SAT-clause
counts, SAT-variable count, grounding time, SAT time, and SAT outcome. These
comments are diagnostic telemetry, not proof evidence.

## Design and provenance

The implementation is independent Rust over Umlaut's backend-neutral
`IncrementalSatService`. Its function-table encoding was selected by the
held-out experiments in
[`experiments/2026-07-29-003-fnt-function-table-models/`](../experiments/2026-07-29-003-fnt-function-table-models/).
The experiment records the prototype, adversarial semantic validation, and
family-disjoint solve evidence. No E, Vampire, or other prover source was
copied into the worker. CaDiCaL remains the already-audited optional MIT
backend described in
[`dependency-packaging-matrix.md`](dependency-packaging-matrix.md).
