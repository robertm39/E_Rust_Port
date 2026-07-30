# Preregistration: real ground-theory branch traces

Bead: `E_Rust_Port-9jt.5.10`

Date frozen: 2026-07-30

## Question

On family-held-out typed arithmetic problems, can an independently replayable
ground difference-logic checker reduce the work of a production-like
propositional branch search or close its finite abstraction without losing a
neutral result? If so, does a small dependency-free native checker retain
enough of the benefit to avoid a mandatory SMT runtime?

A negative or demand-insufficient result completes this evaluation when the
frozen measurements and falsification gates are reported. It does not require
production integration.

## Source corpus and holdout

The only source corpus is the 150-problem TFA division in
`benchmarks/casc_2025_manifest.jsonl`. The manifest identifies the official
CASC-30 presentation, exact source hashes, and family-disjoint
train/validation/test split. A source is arithmetic-bearing when its exact
bytes contain at least one official TPTP arithmetic relation or operation.
Within each partition and source family, the frozen evaluation selects at most
five arithmetic-bearing problems by ascending manifest `size_bytes`, breaking
ties by `problem_id`. Families with fewer than five such problems contribute
all of them. This bounded, metadata-driven rule is applied identically to every
partition and is fixed before any held-out CNF capture.

Development and threshold selection may inspect only train source families.
Validation and test source contents are held out until:

1. the production CNF capture command;
2. deterministic grounding and branch order;
3. supported-fragment classifier;
4. all proof and mutation gates;
5. resource limits; and
6. this advancement rule

are implemented and passing on train. The final tracked corpus records the
manifest identity, source SHA-256, split, family, CNF transcript SHA-256, and
every retained ancestry edge. No source family may cross a partition.

The source problem files are evidence inputs and are not copied into the
tracked experiment or a runtime package.

### Pre-freeze reconnaissance disclosure

Before this preregistration was frozen, repository inventory inspected the raw
text of `ARI519_1`, `ARI632_1`, `ARI708_1`, `NUM861_1`, `SEV422_1`, and one
`ANA143` source to confirm that the CASC TFA partition contained typed
arithmetic and to understand its surface syntax. It also counted arithmetic
tokens across the division. No held-out source was clausified, grounded,
searched, or submitted to a theory checker, and no selection or advancement
threshold uses an observed held-out outcome. The deterministic size rule above
prevents the reconnaissance from choosing favorable held-out cases.

## Production-like trace construction

Each source is parsed and clausified by the repository snapshot's release
`umlaut` binary using `--cnf --tstp-out`. The complete stdout, stderr, exit
status, command, binary hash, source hash, and snapshot identity are retained.
A source that fails parsing or CNF conversion is reported and contributes no
eligible branch.

The experiment parser accepts only complete printed `cnf` or `tcf` records. It
rejects malformed records. Each clause receives a stable identity from the
source hash, printed clause name, role, and canonical clause bytes.

For the bounded abstraction:

- every universally quantified clause variable is replaced consistently
  within its problem by a fresh, typed, canonical ground symbol;
- identical typed ground terms map to identical propositional atoms;
- each normalized clause becomes a propositional clause over its canonical
  ground literals;
- unit propagation and decisions use a fixed lexical atom order, false before
  true;
- tautological clauses are removed, duplicate literals and clauses are
  canonicalized, and an empty clause closes the abstraction;
- at most 256 propositional atoms, 1,024 clauses, 4,096 visited search nodes,
  and 1,024 leaves are retained per problem; and
- crossing a bound terminates that workload as `Unknown`, never as solved.

This is one deterministic ground instance and a bounded abstraction of a real
Umlaut CNF, not a proof of the original first-order problem. “Closed workload”
below means that every propositional branch of this finite abstraction is
closed; source-problem theorem status is reported separately and never inferred
from that result.

Every theory query records:

- source, family, partition, and source hash;
- CNF transcript and clause identity;
- signed literal identity and canonical ground term;
- original variable sort and grounding substitution;
- base, unit-propagation, and decision ancestry;
- incremental context and query sequence numbers; and
- the exact supported constraints submitted to the checker.

## Arithmetic fragment

The eligible fragment is one-sort ground difference logic over `$int` or
`$real`, represented canonically as `x - y <= c` with exact rational `c`.
The distinguished zero term may be either endpoint. Integer bounds are
integral.

Arithmetic terms are flattened exactly through `$sum`, `$difference`,
`$uminus`, and multiplication by a rational constant. Any other typed ground
arithmetic-returning term is an opaque canonical variable; identical terms
share a variable. A relation is eligible only when flattening yields at most
one `+1` and one `-1` symbolic coefficient. General linear, nonlinear, mixed
sort, ill-typed, quantified-after-grounding, NaN/infinite, and unsupported
terms are `Unknown`.

`$less`, `$lesseq`, `$greater`, `$greatereq`, and arithmetic equality are
translated with polarity. Strict integer bounds are reduced by one. Strict real
bounds and disequality are unsupported. Equality is eligible only at positive
polarity and produces both difference directions.

The checker receives the maximal supported, single-sort subset of the selected
arithmetic literals. Every excluded arithmetic literal and its reason remains
in the trace as `Unknown`. A verified `unsat` result for the supported subset
may prune the full branch because an inconsistent subset makes its superset
inconsistent. A `sat` model certifies only the submitted subset: it never
establishes feasibility of the full branch and never closes or skips work.
Mixed supported sorts are not combined in one query. Boolean and
non-arithmetic literals remain in the propositional abstraction but do not
enter a theory query.

## Fixed variants

All variants traverse the same canonical clauses and deterministic decision
order.

1. `no_theory` never performs a theory decision.
2. `native` uses an experiment-only dependency-free Rust Bellman-Ford checker.
3. `process` uses one persistent, shell-free process for the exact pinned Z3
   executable from experiment `2026-07-30-001`.
4. `ffi` uses that experiment's pinned Rust C API prototype and `libz3.so`.

The native checker returns an exact rational model for feasible contexts and a
negative-cycle core for inconsistent contexts. The Z3 variants return named
cores or exact parsed models. All accepted results pass both the Python
verifier and a separately invoked dependency-free Rust replay path. A checker
is called after unit propagation and after each decision only when the context
contains at least two supported arithmetic constraints.

Each source has an exact per-sort constraint-set cache keyed by the canonical
submitted subset. A first occurrence calls the backend and stores only
independently verified evidence. Later occurrences preserve their distinct
search ancestry but reuse the verified verdict and make no backend call.
Unverified, `Unknown`, timed-out, failed, or interrupted work is not cached as
a proof result.

Each backend uses a 5,000 ms per-call limit and a 30-second per-workload branch
budget inside the common controller. Five measured repetitions follow one
warm-up. Verdicts and normalized evidence must be deterministic. Timing
includes dispatch, checking, and evidence extraction; process/driver startup
and shutdown are reported separately.

## Train-only development gate

Before any validation or test source is opened by an experiment script:

1. source and manifest hash tests pass;
2. CNF capture is deterministic on two repeated train runs;
3. parser, grounding, polarity, sort, and difference-classifier tests pass;
4. native models and cores pass exact Python and Rust replay;
5. at least six certificate mutation classes fail closed;
6. native, process, and FFI agree on every common submitted train subset;
7. unsupported/malformed/missing evidence becomes `Unknown`;
8. timeout and cancellation are bounded and never yield a counted decision;
9. branch traces are byte-identical across repeated train generation; and
10. train contains at least 40 eligible queries across at least three source
    families.

If item 10 fails, the result is “insufficient observed demand.” Validation and
test remain sealed, all four variants are still reported on the available
train trace, and production remains unchanged.

## Measurements

For every variant, partition, family, and source workload, report:

- source parse/CNF success and retained clause/atom counts;
- visited nodes, unit propagations, decisions, leaves, theory calls, cache hits,
  pruned nodes, bound terminations, and closed finite abstractions;
- source-problem expected class, without conflating it with abstraction status;
- raw/trusted `sat`/`unsat`/`unknown`, independently verified models/cores, and
  replay coverage;
- unsupported/excluded query and literal reasons and neutral bypass counts;
- call median/p95/total, startup/shutdown, controller wall time, and peak RSS;
- cancellation/timeout outcome and latency;
- executable, shared-library, native/replay driver, trace, and build-tree sizes
  and hashes;
- dynamic runtime dependencies and default-package deltas; and
- deterministic hashes of sources, CNF transcripts, traces, results, and
  evidence.

Neutral workloads are sources with no eligible query after CNF capture. They
must make zero checker calls and traverse exactly the same node/leaf sequence
under every variant.

## Correctness and falsification gates

Any of the following makes the affected backend inconclusive and forbids
production use:

1. a counted result lacks a complete ancestry chain;
2. an accepted core has no exact negative cycle;
3. an accepted model violates a submitted constraint;
4. Python and Rust replay disagree;
5. a mutation is accepted;
6. a supported common query has disagreeing trusted verdicts;
7. unsupported, timeout, interruption, malformed, or failed work is trusted;
8. a neutral workload's search sequence changes;
9. a backend skips a branch without verified `unsat`; or
10. a default build or package acquires an optional backend dependency.

## Advancement rule

Production remains off unless every correctness gate passes. A follow-up for
the native checker is justified only if combined held-out validation and test:

1. contain at least 40 eligible queries from at least two source families;
2. replay 100% of trusted decisions in both independent checkers;
3. prune at least 20 nodes and at least 5% of eligible visited nodes;
4. close at least one finite abstraction or reduce visited nodes by at least
   10% on three separate source workloads;
5. lose no closed abstraction and change no neutral trace;
6. keep native p95 call latency at or below 0.25 ms;
7. keep native release-package growth at or below 256 KiB with no new runtime
   dependency; and
8. bound cancellation by one second.

A production SMT-backend follow-up additionally requires FFI p95 at or below
2 ms, process p95 at or below 10 ms, no held-out benefit loss relative to
native, explicit removable packaging, and a separately approved deployment
plan. Passing these thresholds does not authorize integration in this
experiment.

No post-hoc corpus, split, limit, fragment, threshold, or result-name change is
permitted after held-out execution begins.
