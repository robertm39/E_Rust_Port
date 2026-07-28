# Soundness validation gates

Umlaut's offline validation gate is
[`tools/validation/validate_tptp_solution.py`](../tools/validation/validate_tptp_solution.py).
It is intended for regression experiments, release evidence, proof-storage
changes, and new result-producing engines. It does not run inside the
competition-time proof search.

## Verdict policy

The gate is positive-only:

- `verified`: the problem/status boundary and output structure passed, and the
  configured independent semantic checker emitted `SZS status VerifiedGood`;
- `rejected`: the claim contradicts the problem status, the output is
  malformed, or the checker emitted `VerifiedBad` or otherwise failed its
  positive contract;
- `coverage_gap`: the output object or checker is missing, or the checker
  reported `Unknown`/`Timeout`;
- `not_applicable`: the final SZS status makes no proof or model success claim;
- `error`: the gate input or checker configuration is invalid.

The process exits 0 for `verified` and `not_applicable`, 1 for `rejected`, 2
for `coverage_gap`, and 3 for `error`. `--allow-coverage-gap` is available for
inventory runs, but consumers must still inspect the JSON verdict.

## Layers

1. The final SZS status is compared with the problem's declared `% Status :`.
2. SZS output blocks must be non-nested, have matching start/end types, and
   contain exactly one object appropriate to the success claim.
3. Refutations must contain an annotated `$false` formula.
4. Optional syntax and required semantic checker commands run as argument
   vectors without a shell.
5. Only semantic `VerifiedGood` is accepted.

The checker command is deliberately configurable rather than a bundled
dependency:

```text
python3 tools/validation/validate_tptp_solution.py \
  problem.p solution.s \
  --proof-command-json \
  '["/opt/proofcheck/proofcheck","-p","{problem}","{artifact}"]' \
  --report validation.json
```

The placeholders are absolute paths to the original problem, full solution,
and extracted proof/model artifact.

## Current coverage

Experiment
[`2026-07-27-004`](../experiments/2026-07-27-004-soundness-validation-gates/)
independently verifies representative FOF `Theorem` and CNF `Unsatisfiable`
proofs with ProofCheck 1.0. A corrupted input leaf produces `VerifiedBad`, and
a forged theorem on a known counter-satisfiable problem is rejected before the
checker runs.

Experiment
[`2026-07-28-001`](../experiments/2026-07-28-001-proof-checker-coverage/)
adds representative FOF `ContradictoryAxioms` coverage with GAPT 2.20. GAPT
reports `VerifiedGood` for the original proof and `VerifiedBad` after a
derived clause is changed from `p(a)` to `q(a)`; the gate returns `verified`
and `rejected`, respectively. Umlaut also now preserves question annotation
and conjecture negation as explicit archived proof steps rather than nesting
them below later formula preprocessing.

The following gaps are intentional and machine-visible:

- TFF is not positively verified: ProofCheck abstains, GAPT 2.20 reports
  `Unknown`, and Nörgler 1.1 reaches its FOF-only conjecture-negation routine
  and returns `Error`;
- THF is not positively verified: GAPT 2.20 reports `Unknown`, and Nörgler
  1.1 reaches the same FOF-only routine and returns `Error`;
- `Satisfiable` and `CounterSatisfiable` saturation paths do not emit a TPTP
  interpretation, so they cannot be independently evaluated as models.

A new prover mode must not treat a matching second-solver status as proof or
model verification. It must preserve enough provenance to emit a checkable
artifact and configure a checker whose positive result is independent of the
claiming code path.

## Test and release integration

The standard-library controller tests are in
[`tools/validation/test_validate_tptp_solution.py`](../tools/validation/test_validate_tptp_solution.py).
The comprehensive Linode workflow runs them after the native Rust gates and
archives `solution-validation-test.txt` plus timing.

Do not commit third-party checker binaries. Pin release identity and hashes in
an experiment, verify them before execution, retain licenses with ignored raw
artifacts, and keep external processes outside the Umlaut runtime package.
