# Restricted integer-induction findings

## Decision

Do not integrate integer-induction schema generation into production Umlaut.
The fixed prototype advanced one of two untouched targeted test problems, but
advanced none of the five trigger-positive CASC-30 problems and did not reduce
their search work by the preregistered threshold. The one candidate-only test
proof also remained outside ProofCheck 1.0's typed-arithmetic parser coverage.
The final preregistered verdict is
`defer_production_integer_induction`.

This experiment changes no production prover or default. It retains a
reproducible, proof-explicit capability prototype and identifies arithmetic
semantics, guard preservation, and hypothesis strength as prerequisites for a
future induction design.

## Architecture conclusion

Umlaut's existing `--preinstantiate-induction` option instantiates clauses
that already have induction shape; it does not synthesize an induction
principle. The evaluated prototype therefore remained an external TPTP source
transformation. It added at most one named axiom of the form:

```text
( P(b)
  & ! [N: $int] :
      (( $greatereq(N,b) & P(N) ) => P($sum(N,1))) )
=>
! [N: $int] :
  ( $greatereq(N,b) => P(N) )
```

Structural induction was deliberately excluded. Umlaut does not yet parse
algebraic-datatype declaration roles or enforce free, exhaustive datatype
semantics. Treating ordinary constructor symbols as an inductively generated
datatype would make an induction schema unsound.

The first pre-search syntax gate also established that Umlaut recognizes
`$int` numerals but does not predeclare the standard arithmetic types used
here. The harness consequently added redundant type declarations for `$sum`,
`$difference`, `$product`, `$greatereq`, and `$lesseq` to both treatments.
They add no clauses and no arithmetic evaluation. This parser-only correction
was frozen before any search outcome.

## Frozen population and trigger

The trigger accepted a single lower-bounded integer conjecture with a literal
bound and quantifier-free property. It accepted universal implication and
negated-existential spellings, including a `$lesseq(b,N)` lower-bound guard.
No outcome participated in selection.

The immutable CASC-30 audit accepted 5 of 2,901 problems:
`SWC434_1`, `SWC435_1`, `SWC448_1`, `SWC449_1`, and `SWC462_1`. All five had
the preregistered predecessor/successor recurrence proxy, giving static proxy
precision 5/5. All are train-split members of one SWC family, so this transfer
set is not a family-held-out efficacy population. Its observed solve yield was
0/5 for both treatments.

Audit report:
`2a2973588148d621252161c3db8c1c1f8df75ba9b4bc0ea99bdf395d52cfb172`.

## Soundness and execution gates

Every one of the 11 materialized problem pairs passed:

1. generator reconstruction;
2. the independent `verify_schema.py` recognizer;
3. Umlaut syntax parsing; and
4. Umlaut clausification.

The fixed release binary and source snapshot were identical across all 40
runs. There were no missing telemetry records, external timeouts,
contradictory statuses, or lost baseline solves.

The proof adapter parsed every source and proof leaf structurally, checked
alpha-equivalence while preserving the logical proof fields, and confirmed
six input leaves in each of the two repeated candidate proofs. ProofCheck 1.0
itself passed all 117 self-tests, but returned `Unknown` for both proofs. Its
parser rejects applied interpreted symbols in these typed proof formulas,
including `$greatereq(...)`; the checker-facing proof was not changed by
renaming those logical symbols. The preregistered proof gate therefore fails
closed at 0/2 verified claims. Proof-validation report:
`d63667edee9100b215b3207bb6d2c4e02799d2480488b01f709f1f5a5e365455`.

## Search results

| Phase | Problems | Repetitions | Baseline | Induction | Candidate-only |
| --- | ---: | ---: | --- | --- | --- |
| calibration | 2 | 1 | 2 `GaveUp` | 2 `Theorem` | `function_fixed_point`, `predicate_chain` |
| validation | 2 | 2 | 4 `GaveUp` | 4 `GaveUp` | none |
| untouched targeted test | 2 | 2 | 4 `GaveUp` | 2 `Theorem`, 2 `GaveUp` | `conjunctive_invariant` |
| CASC transfer | 5 | 2 | 10 `ResourceOut` | 10 `ResourceOut` | none |

The successful conjunctive test result reproduced twice. The other untouched
test, `equality_context`, failed in both treatments because the generated
hypothesis `wrap(f(N)) = wrap(z)` is too weak to establish the recursive
step's needed `f(N) = z`; this is a concrete hypothesis-strengthening gap.

The nonzero-bound validation problem used `$lesseq(3,N)`. The generator
normalized that guard to `$greatereq(N,3)`. Those forms are equivalent under
standard integer semantics, but Umlaut currently searches over them as
unrelated uninterpreted predicates. This exposes a practical guard-spelling
and arithmetic-semantics dependency rather than evidence for production
induction.

## Clause growth and transfer cost

The schema clausified to three additional clauses on ten problems. The
conjunctive test property produced eight additional clauses:

| Population | Baseline-to-induction clause ranges | Delta |
| --- | --- | ---: |
| calibration | 5 to 8; 4 to 7 | +3 each |
| validation | 2 to 5; 4 to 7 | +3 each |
| targeted test | 6 to 14; 5 to 8 | +8; +3 |
| CASC transfer | 11 to 14; 11 to 14; 6 to 9; 10 to 13; 12 to 15 | +3 each |

Across paired CASC transfer runs, the induction/baseline median ratios were:

- CPU: 0.999858;
- generated clauses: 1.003409;
- processed clauses: 1.032494;
- high-water clause count: 1.025331; and
- maximum resident pages: 1.016236.

Thus the candidate neither met the 20% generated-clause reduction alternative
nor showed a transfer solve. The fixed decision also fails because only one of
two targeted tests advanced and the independent checker could not cover its
typed-arithmetic proof.

## Reproduction

`run.py` reconstructs each phase from the frozen fixtures or
`selected-problems.json`, validates both materialized treatments, and resumes
only against an exact matching contract. `analyze.py` rejects incomplete
matrices, contract/hash mismatches, missing telemetry, and contradictory
statuses before applying the preregistered decision. `verify.py` independently
audits every reproducible candidate-only test proof and records checker
coverage gaps without weakening them into successful validation.

The canonical final summary contains 40 runs and has ID:
`03b11b9502b1b59b4698fde361afd2a429dbe1610785cbe5dab170eab3ceec82`.
The ignored complete evidence archive is
`.artifacts/experiments/2026-07-29-022-integer-induction-schema/ind-022-complete.tar.gz`;
its SHA-256 is
`344351f8d634e51b26feede295970213df391b0d67e183f29d1798dd0e8f8ecf`.
