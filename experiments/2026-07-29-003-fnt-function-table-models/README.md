# Typed finite-model function tables

Bead: `E_Rust_Port-9jt.6.8`

## Question and preregistration

Can an isolated finite-model worker extend the earlier function-free FNT
prototype with sound positive-arity function tables, native many-sorted
domains, and incremental/domain-aware SAT solving, then contribute at least
one independently verified held-out model that unchanged Umlaut does not find
at the same per-problem resource limit?

This section is preregistered before the new worker is run on the frozen
validation or test partitions.

## Scope

The worker consumes the typed TSTP CNF emitted by Umlaut's existing
clausifier. It supports:

- first-order clauses over uninterpreted nonempty native TFF sorts;
- recursively nested variables, constants, and positive-arity functions;
- predicates and equality/disequality whose argument and result types are
  explicit and consistent; and
- conjectures after Umlaut's normal negation and clausification.

It rejects interpreted arithmetic and other interpreted nonlogical symbols,
distinct objects, polymorphic or higher-order types, type variables, and any
term or symbol whose type cannot be established consistently. Rejection must
produce `SZS status Inappropriate`. Exhausting finite bounds must produce
`SZS status GaveUp`; a finite search never establishes unsatisfiability.
Timeouts, encoding limits, solver protocol errors, incomplete SAT models,
and internal model-check failures must make no success claim.

The earlier experiment in
`experiments/2026-07-28-011-fnt-finite-model-prototype` remains immutable.

## Encoding

For a configured maximum cardinality, every native sort has a prefix-active
domain. SAT assumptions select the exact active prefix for each attempted
domain-size vector. The global encoding contains:

- one-hot rows for every constant and every positive-arity function-table
  input tuple;
- Boolean predicate-table rows;
- clauses that force constants and active function rows to return an active
  value;
- guarded, one-hot values for nested ground terms, linked to the shared
  function tables; and
- guarded truth variables for predicate and equality literals.

Universal clause instances are generated only when their typed variable
assignment first becomes reachable. They remain valid for later, incomparable
size vectors because every instance is guarded by its domain-activity
assumptions. A single long-lived instance of Umlaut's statically linked
CaDiCaL 3.0.1 service receives new permanent clauses and successive
assumption queries. Each bound records new and cumulative grounding clauses,
active and cumulative ground instances, propositional variables and clauses,
clause insertion time, SAT time, result, and complete-model size.

The initial implementation favors auditability over aggressive compactness.
Every SAT assignment is checked against the typed clause set in Python before
rendering. Every rendered success is then checked independently against the
original, unclausified input through the repository's positive-only solution
validator and pinned Vampire model checker.

## Corpus and split discipline

Use the exact CASC-J11 FNN/FNQ manifest and family-level split frozen by the
earlier experiment: 158 training problems from 17 families, 30 validation
problems from four families, and 62 test problems from four families. The old
inventory and outcomes may guide implementation because they are already
known training evidence. Validation is used to select at most one fixed
configuration. Test remains unopened until that selection and all soundness
gates are frozen.

Add hand-written typed and untyped fixtures for function tables, nested
functions, multiple native sorts, conjectures, inconsistent types, unsupported
interpreted symbols, bound exhaustion, resource exhaustion, and solver
protocol failures. Fixtures establish semantics and negative behavior; they
do not count as held-out performance evidence.

Compare the selected worker with unchanged Umlaut `--auto` at an equal
10-second soft CPU limit and 15-second controller wall limit per problem.
Pinned Vampire may be reported as context but is not the uniqueness baseline.
The finite worker searches cardinalities one through three per native sort,
with no more than 2,048 size vectors and 5,000,000 cumulative ground
instances per problem.

## Validation gates

Before held-out evaluation:

1. unit and integration tests must cover parsing, typing, function-table
   linkage, nested-term evaluation, many-sorted bounds, incremental guarded
   grounding, model rendering, and all fail-closed statuses;
2. independently validate models for at least one unary function, one nested
   function, and one native two-sort problem;
3. reject single-change corruptions of a function-table row, predicate row,
   constant value, native-sort domain, status, and declared type;
4. cross-check incremental and fresh encodings on small exhaustive fixtures;
5. retain complete per-bound telemetry and reproducible hashes; and
6. pass the repository's comprehensive Ubuntu quality gate because the
   experiment exercises Umlaut and the production CaDiCaL service.

## Decision rule

Production integration is permitted only if all emitted successes are
independently `VerifiedGood`, every corruption and malformed input is
rejected, incremental and fresh solve sets agree on exhaustive small
fixtures, bounded failures remain fail-closed, and the frozen selected
configuration contributes at least one independently verified validation or
test solve not reported by unchanged Umlaut at the equal resource limit.

Otherwise the worker remains experimental. Report coverage, resource, or
encoding blockers and file follow-up Beads rather than wiring it into the
default prover.
