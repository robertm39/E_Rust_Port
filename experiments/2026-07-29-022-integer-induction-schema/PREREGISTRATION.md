# Preregistration

Recorded before building the candidate binary or running either treatment.

## Question

Can one deliberately restricted, proof-explicit integer-induction schema
prove recursive integer properties beyond equal-budget ordinary saturation,
and does the trigger transfer beyond constructed examples without
unacceptable clause growth?

## Architecture boundary

The prototype is a TPTP source transformation rather than a production
inference. It generates at most one named axiom for one conjecture. This keeps
the induction assumption visible to Umlaut and independent proof tools while
avoiding an unsound attempt to treat arbitrary first-order constructors as an
inductive datatype.

Structural induction is not prototyped because Umlaut does not currently
parse the TPTP algebraic-datatype declaration roles or enforce datatype
generation semantics. Treating ordinary `nil`/`cons` declarations as a free,
exhaustive datatype would be unsound.

## Trigger

The fixed trigger accepts a TFF conjecture equivalent by one syntactic step
to:

```text
! [X: $int] : ( $greatereq(X,b) => P(X) )
```

The equivalent guard spelling `$lesseq(b,X)` is accepted. The source may
instead use:

```text
~ ? [X: $int] : ( $greatereq(X,b) & V(X) )
```

in which case `P(X)` is `~V(X)`. The integer bound must be a literal, `P`
must be quantifier-free, and the bound variable must occur in `P`. Nested
quantifiers, multiple integer binders, strict bounds, symbolic bounds, and
noninteger targets are rejected. No problem outcome participates in
selection.

## Population

There are two populations.

1. Six committed targeted problems exercise predicates, equality, a nonzero
   bound, conjunction, and a recursive function context. Two are calibration,
   two are validation, and two are untouched test examples. One additional
   validation problem is a trigger-positive negative control with no base or
   step facts.
2. The transfer population is every CASC-30 presented problem accepted by the
   trigger. `audit.py` scans the immutable manifest and corpus, records
   predecessor/successor-recursion syntax as an outcome-blind relevance
   proxy, and refuses a manually selected subset.

CASC-30's relevant SWC/DAT problems all belong to train families. Therefore
the transfer population is explicitly not called held out. It can reject
production integration, but it cannot by itself establish a family-held-out
gain.

### Static audit result

The frozen audit accepted five of 2,901 problems: `SWC434_1`, `SWC435_1`,
`SWC448_1`, `SWC449_1`, and `SWC462_1`. All are train-split SWC problems and
all five contain the recurrence proxy. The audit report ID is
`2a2973588148d621252161c3db8c1c1f8df75ba9b4bc0ea99bdf395d52cfb172`.
`selected-problems.json` pins their source hashes and generated schema
identities. This count was recorded before candidate build or execution.

### Pre-outcome parser correction

The first syntax/clausification gate, before any proof search, found that
Umlaut parses `$int` numerals but does not predeclare the types of `$sum`,
`$difference`, `$product`, `$greatereq`, or `$lesseq`. Every accepted CASC
problem and every original fixture therefore exited with a type error.

The experiment now prepends the standard integer type declarations for those
five interpreted symbols to both treatments. These are redundant under TPTP
arithmetic semantics, generate no logical clause, and do not add arithmetic
evaluation. The induction treatment still differs only by its one named
schema. This correction was frozen before observing a baseline or induction
search outcome.

## Treatments

- `baseline`: the original problem plus redundant standard arithmetic type
  declarations;
- `induction`: the same prepared problem plus exactly one generated schema.

Both treatments use the same fixed completion-shaped heuristic, KBO6, no
literal selection, full forward demodulation, and presaturation
simplification. The problem text is the only treatment difference.

## Budgets and repetitions

- targeted calibration: 2/4 seconds, one repetition;
- targeted validation: 4/6 seconds, two repetitions;
- targeted test: 8/10 seconds, two repetitions;
- CASC transfer population: 8/10 seconds, two repetitions.

The first number is Umlaut's soft CPU limit and the second its hard CPU limit.
Per-run memory is 1536 MiB. Test and transfer runs request TSTP proof objects.

## Measures

Primary:

- reproducible candidate-only targeted test solves;
- candidate-only and lost CASC transfer solves;
- generated/processed/input clauses and CPU;
- schemas generated, trigger rejections, and recurrence-proxy precision;
- generated schema count and clausified clause growth.

Secondary:

- proof length, maximum resident pages, paramodulations, and rewrite steps;
- calibration/validation behavior, reported separately from test.

## Soundness

Every generated schema must pass:

1. token-level reconstruction by `schema.py`;
2. an independent recognizer in `verify_schema.py`;
3. Umlaut parsing and clausification.

Every reproducible candidate-only proof claim must pass the repository proof
adapter, TPTP solution gate, and ProofCheck 1.0. The checker-facing problem is
the augmented problem, and no logical proof field may be edited.

## Decision rule

Production integration is justified only if:

1. both targeted test examples are reproducible candidate-only solves;
2. all generated schemas and all claimed proofs verify;
3. there is no contradictory status or lost baseline solve; and
4. at least one nonconstructed transfer problem is a reproducible
   candidate-only solve, or the transfer population shows a repeatable
   20% reduction in generated clauses on common solves without a CPU
   regression.

If only constructed examples advance, retain the experiment as a validated
capability prototype but defer production integration. If targeted examples
also fail, document that the path presently lacks leverage.
