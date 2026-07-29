# Preregistration

## Question

Which inference and simplification mechanisms exposed by current reference
provers are semantically direct, partial, absent, or merely unreachable in
Umlaut, and does the highest-evidence low-risk candidate warrant a production
change?

This study treats source names as navigation only. A claim of direct support
requires:

1. a semantic description and soundness preconditions;
2. a production call route or preprocessing route;
3. a proof-ancestry operation when the transformation affects a proof;
4. a focused executable witness; and
5. a command-line reachability classification.

An absent spelling is not by itself a gap. A rule may be intentionally
subsumed by another calculus, and a default-off rule is not missing.

## Frozen baseline and license boundary

The source baseline is commit
`d21e1a7d22d6d2d2ef4cc9ace95a9b9404b5e2fa`. The audit may read and test
Umlaut and the bundled E compatibility source. Vampire is a clean-room
reference boundary: only its official public documentation and the
integrity-pinned Linux executable interface may be used. Vampire implementation
source must not be inspected or copied. No reference binary or reference code
becomes a product dependency.

All Rust builds, Rust tests, prover runs, timings, and proof checks execute on
one ephemeral Ubuntu 24.04 worker. Local work is limited to static inspection,
Python controller tests, corpus extraction, and result analysis.

## Frozen capability classifications

The committed `capability-matrix.json` is the audit contract. Its statuses
mean:

- `direct`: Umlaut implements the same semantic operation with a production
  route and focused witness;
- `partial`: Umlaut has useful neighboring operations but not the complete
  stated rule;
- `library_only`: a tested low-level constructor exists but has no production
  dispatch route; the report must state whether that is inherited/intentional
  or a candidate defect;
- `missing`: no equivalent production rule was found;
- `owned_elsewhere`: the gap is already assigned to a more specific Bead and
  is not a candidate here.

The controller must fail if a direct row loses its focused test, route marker,
or proof-operation marker, or if an absence boundary unexpectedly acquires a
named route without the matrix being reviewed. Remote execution runs every
focused Rust witness individually and requires exactly one matching passing
test per filter.

Ordered factoring is explicitly `library_only`: its constructors and
`DC_ORDERED_FACTOR` ancestry are tested, but no Rust production caller exists.
The bundled E compatibility search likewise calls equality factoring rather
than `ComputeAllOrderedFactors`; this inherited dormant utility is not promoted
above a production-reachable candidate merely because it exists.

The audit corrects one stale prior classification before benchmark data:
`clause_local_rw` uses an oriented negative equality inside a clause to rewrite
the other literals, is called from selected-clause forward modification when
`local_rw` is enabled, records `DC_LOCAL_REWRITE`, and is reachable through
`--local-rw=true`. This is direct inner rewriting, not merely an adjacent
mechanism. The option is stored under the historical higher-order configuration
group but its clause implementation and selected-clause call route are not
restricted to THF.

## Frozen shortlist

Only these three items are shortlisted:

1. **Selective local/inner rewriting** — direct and already production-ready,
   but default-off in all generated schedules. It has the smallest
   implementation and proof risk and is the only item evaluated in this study.
2. **UR-resolution specialization** — absent as a named specialization.
   Ordinary ordered predicate resolution remains complete for the intended
   first-order role, so this is an efficiency candidate rather than a
   capability blocker.
3. **Term-algebra constructor rules** — no dedicated constructor
   distinctness, injectivity, exhaustiveness, or acyclicity calculus was found.
   Umlaut's generic equality reasoning and injectivity-definition
   preprocessing are not equivalent. This has greater parser/type/calculus
   scope and is not prototyped without a stronger corpus signal.

Non-unit subsumption demodulation and constrained forward ground joinability
remain partial, but experiment `2026-07-28-008-stronger-redundancy` already ran
752 staged coordinates over the strongest existing approximations without a
held-out default-change signal. They are recorded in the matrix but do not
displace the shortlist.

Theory instantiation and arithmetic simplification are `owned_elsewhere`;
experiment `2026-07-29-005-arithmetic-qe-oracle` and its Beads own that
architecture.

## Focused local-rewriting witness

The unit witness
`negative_oriented_literal_rewrites_other_literals` constructs

```text
f(a) != a | g(f(a)) = c
```

and requires local rewriting to produce

```text
f(a) != a | g(a) = c
```

while preserving the negative equality as the local rule, invalidating stale
ordering flags, refreshing clause weight, and recording
`DC_LOCAL_REWRITE`. A proof-control witness separately requires the option to
trigger from the selected-clause forward-modification route. CLI parsing and
control construction are covered by the existing option/control tests in the
matrix.

## Candidate-blind CASC evaluation

The candidate reuses the immutable CASC-30 manifest and exact family-aware test
selection from experiment `2026-07-28-008-stronger-redundancy`: six FEQ, six
FNE, two EPS, and six UEQ problems. Baseline outcomes from that earlier study
are known, but no `local_rw=true` result on these coordinates has been
inspected. The candidate, thresholds, arguments, and analysis are frozen
before its first run, so this is a candidate-blind reuse rather than a claim
that the corpus itself is unseen.

Both configurations use:

```text
--expert-heuristic=(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))
--term-ordering=KBO6
--forward-demod-level=2
```

The only difference is:

- baseline: historical `local_rw=false`;
- candidate: `--local-rw=true`.

Every coordinate runs twice at 5/7-second soft/hard and 20/23-second
soft/hard budgets. The harness records the exact source, manifest, problem,
binary, strategy, and command hashes; complete stdout/stderr hashes; SZS
status; external timeout; wall time; aggregate search telemetry; and isolated
maximum resident pages. Proof objects are enabled.

The report compares reproducible solve coverage and paired all-run and
common-solved medians for CPU, generated clauses, processed clauses,
high-water clauses, term storage, rewrite steps, and resident memory. It also
reports exact coordinates whose telemetry differs and proof objects containing
`inference(local_rw,...)`.

FNE and EPS are overhead controls: local equality rewriting should be inert
there unless preprocessing creates an eligible equality clause.

## Correctness gates

The evaluation is invalid unless:

1. every focused matrix witness passes on Ubuntu;
2. the release all-feature build succeeds;
3. every coordinate completes or reaches its frozen external timeout without a
   harness error;
4. no baseline/candidate pair has contradictory proof/non-proof polarity;
5. every reproducible candidate proof claim selected for final reporting is
   independently accepted by the existing first-order ProofCheck 1.0 pipeline;
6. a second invocation resumes every coordinate without changing any result
   file; and
7. final formatting, strict all-target/all-feature Clippy, library and
   integration tests, and optimized all-feature build pass if production code
   changes.

## Decision rule

Local rewriting may enter a default first-order schedule only if all correctness
gates pass, there is no reproducible baseline-only solve, maximum RSS is at
most 1.05 times baseline, and either:

1. it contributes at least two reproducible candidate-only solves; or
2. among at least four common-solved problems, median generated clauses are at
   most 0.90 times baseline, median CPU is at most 1.02, and median high-water
   clauses are at most 1.02.

The candidate must also show an observable behavior effect: at least one exact
search coordinate differs in generated/processed/rewrite counts or one checked
proof contains `local_rw`. Passing only through timing noise is insufficient.

If the rule misses this gate, production and schedules remain unchanged.
Missing UR-resolution and term-algebra rules will not be implemented in this
study; they remain evidence-ranked follow-up candidates. If local rewriting
passes, the only permitted production change is the smallest selective
schedule/configuration change justified by the measured categories, followed
by a fresh held-out confirmation under the same correctness gates.
