# Preregistration

## Question

Can a small SAT encoding outperform Umlaut's current recursive first-order
subsumption matcher on a reproducible regime of real saturation calls, and does
the same match set expose enough subsumption-resolution opportunities to justify
a proof-producing production follow-up?

## Provenance and semantic contract

The implementation is derived from the published constraint definitions in:

- Rath, Biere, and Kovács, *First-Order Subsumption via SAT Solving*,
  FMCAD 2022; and
- Coutelier, Rath, Rawson, Biere, and Kovács, *SAT Solving for Variants
  of First-Order Subsumption*, Formal Methods in System Design, 2025.

No reference-prover implementation is copied, translated, linked, or included.
The repository's ignored VIRAS implementation is outside the provenance
boundary and must not be inspected.

For a side clause `S` and main clause `M`, the experiment materializes every
one-way literal match as a Boolean choice carrying its partial substitution.
Pairwise clauses reject choices that bind the same side variable to different
main terms.

Ordinary subsumption requires:

1. exactly one same-polarity choice for every side literal;
2. at most one side literal mapped to each main literal; and
3. pairwise-compatible partial substitutions.

Subsumption resolution requires:

1. exactly one same- or opposite-polarity choice for every side literal;
2. at least one opposite-polarity choice;
3. all opposite-polarity choices mapped to one main literal;
4. no same-polarity choice mapped to that resolution literal; and
5. pairwise-compatible partial substitutions.

The declared fragment is duplicate-free, first-order clauses after Umlaut's
normal simplification and subsumption-order preparation. Higher-order cases are
not silently approximated.

## Corpus and separation

The immutable
[`benchmarks/casc_2025_manifest.jsonl`](../../benchmarks/casc_2025_manifest.jsonl)
supplies the same family-separated CASC-30 split used by earlier search
experiments.

- Calibration: 24 train problems, six each from FEQ, FNE, EPS, and UEQ.
- Validation: 24 validation problems with the same category quotas.
- Held-out test: 20 test problems: six FEQ, six FNE, both EPS problems, and
  six UEQ.

No family crosses splits. Each problem uses the fixed KBO6,
`5*Refinedweight + 1*FIFO` configuration with full forward demodulation.
Calibration and validation receive five soft CPU seconds per problem; test
receives ten. Runs use one process per problem and a 1,536 MiB memory limit.

## Workload sampling

Only non-unit, first-order calls reaching the bank-aware clause checker are
eligible. Each process records:

- its first 256 eligible calls;
- every 997th later eligible call; and
- every 31st later call whose side has at least four literals and whose main
  clause has at least six literals;

up to 2,048 records per problem. This rule is fixed before seeing timings.
Every record contains canonical structural clause encodings, the existing
checker result and latency, both Boolean outcomes and latencies, choice/CNF
sizes, substitution-binding count, and a content digest.

Sampling is for crossover estimation, not a claim about unweighted call
frequency. Reports must provide both record-level and problem-balanced
summaries.

## Correctness and falsification

The following are hard gates:

1. every sampled ordinary SAT result agrees with Umlaut's current checker;
2. experiment Rust tests cover shared bindings, swapped equalities,
   multiplicity, complementary-match uniqueness, and coherence;
3. an implementation-independent Python oracle exhaustively enumerates small
   clauses and differentially checks at least 10,000 seeded randomized pairs;
4. corrupting one expected oracle result makes the validation command fail;
5. malformed records, duplicate digests with different payloads, unknown SAT
   outcomes, or missing split coordinates fail analysis.

Subsumption-resolution results are treated as prospective simplifications, not
as proof-producing Umlaut inferences. They cannot be enabled in production
without ancestry, proof output, and independent proof-checker coverage.

## Measurements

For each split and clause-size regime the report includes:

- agreement, positive/negative results, source and target literal counts,
  variable counts, match choices, bindings, CNF clauses/literals;
- median, p90, p95, p99, maximum, and aggregate latency for the recursive
  checker and Boolean encodings;
- process maximum RSS and tracked prototype/source size;
- ordinary-subsumption success counts and prospective
  subsumption-resolution literal-cut counts;
- per-problem and per-category distributions; and
- full-prover status, search telemetry, CPU, wall time, and RSS for any
  selected dispatch policy.

Because ordinary SAT checking is semantically equivalent, its pruning gain is
necessarily zero relative to the existing checker. Any additional pruning is
reported only for subsumption resolution and remains prospective.

## Dispatch selection and decision rule

Calibration may consider thresholds over side literal count, main literal
count, and positive match-choice count. A threshold is eligible only with at
least 200 records from at least six problems, zero disagreements, aggregate
SAT/checker time at most `0.80`, p95 at most `0.90`, and estimated peak
per-call encoding storage below 256 KiB.

The single best eligible calibration threshold is frozen by canonical JSON and
tested once on validation. It advances only if validation has at least 200
records from at least six problems, zero disagreements, aggregate time at most
`0.90`, p95 at most `0.95`, and no problem-balanced median regression above
`1.10`.

Only an advancing validation policy may be run as a held-out full-prover
dispatcher. Production integration requires all held-out proof/model statuses
to agree, every reproducible proof to pass the independent proof gate, no
baseline-only solve, median CPU at most `0.97`, p95 CPU at most `1.00`,
maximum RSS at most `1.05`, and non-increasing generated/high-water clauses.

If no policy advances, Umlaut's current matcher remains unchanged and the Bead
may close with a documented negative result. A high subsumption-resolution
activation rate may justify a separate proof-producing implementation Bead,
but cannot override the ordinary-dispatch gate.
