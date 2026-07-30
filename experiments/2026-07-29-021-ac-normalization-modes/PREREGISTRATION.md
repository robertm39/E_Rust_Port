# Preregistration

Recorded before building the candidate release binary or observing any
candidate outcome.

## Question

Does Umlaut's existing AC canonical equality and proof-producing AC
redundancy handling reduce term structure or inference cost and improve
family-held-out algebraic solves? This experiment does not add AC rewriting,
matching, unification, indexing, or an unproved joinability criterion.

## Population and split

The population is the complete set of CASC-30 UEQ and FEQ presented problems
that contain, after comments and whitespace are removed:

1. an explicit binary commutativity equality `f(X,Y)=f(Y,X)`; and
2. an explicit binary associativity equality
   `f(f(X,Y),Z)=f(X,f(Y,Z))`

for the same unquoted symbol and variable spelling. Selection is purely
syntactic and independent of prover outcomes. Both equality orientations are
accepted by the audit. `selected-problems.json` records the 26 problems found.

The repository's immutable family partition is retained:

- calibration: 14 train problems from KLE and LAT;
- validation: 10 validation problems from NUM, RNG, and SWV;
- test: 2 test problems from REL.

The small test set is a preregistered limitation. No train or validation
problem may be moved into test, and no problem may be added based on a run
outcome.

## Treatments

All treatments use the same completion-shaped given-clause heuristic, KBO6,
no literal selection, disabled equality factoring, full forward demodulation,
and presaturation simplification. The sole treatment variable is:

- `none`: `--ac-handling=None`;
- `discard_all`: `--ac-handling=DiscardAll`;
- `keep_units`: `--ac-handling=KeepUnits`;
- `keep_orientable`: `--ac-handling=KeepOrientable`.

These are existing modes. `DiscardAll` is Umlaut's default, but `none` is the
causal baseline for all AC-specific contraction.

## Budgets and repetitions

- calibration: one repetition, 4-second soft / 6-second hard CPU limit;
- validation: two repetitions, 8-second soft / 10-second hard CPU limit;
- test short: two repetitions, 5-second soft / 7-second hard CPU limit;
- test larger: two repetitions, 20-second soft / 23-second hard CPU limit.

Every test run requests a TSTP proof object. The complete matrix contains 168
runs. Per-run memory is 1536 MiB. Coordinates are executed in a
contract-derived shuffled order and may run in parallel.

## Measures

Primary:

- reproducible expected-status solves and mode-unique/lost held-out solves;
- AC equality checks and successful hits;
- top-level normalizations, input nodes, normalized nodes, and flattened nodes;
- generated and processed clauses, paramodulations, rewrite steps, CPU, clause
  high-water, and resident pages.

All global telemetry is interpreted as a per-run delta. A missing telemetry
record is not zero.

## Soundness

Every reproducible larger-budget test proof claim from every mode will be
passed through the repository proof adapter, the TPTP solution gate, and
ProofCheck 1.0 after ProofCheck passes all 117 self-certification tests.
Logical proof fields may not be changed.

## Decision rule

Further AC-aware indexing or joinability work is justified only if all claimed
held-out proofs verify and at least one non-`none` mode:

1. has a held-out solve not obtained by `none`, with no contradictory status;
   or
2. loses no held-out solve, records nonzero AC hits and flattening, and reduces
   paired CPU or generated clauses by at least 10%.

The two-problem test set makes this an exploratory rather than definitive
advance decision. A failure to meet the rule rejects production changes from
this Bead and defers costlier AC matching/indexing/joinability.
