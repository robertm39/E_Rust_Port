# Independent validation gates

## TPTP solutions

`validate_tptp_solution.py` turns proof and model checking into a
positive-only release gate. It checks the final SZS status against the
problem's declared status, extracts and validates SZS output framing, and then
requires an independent semantic checker to report `SZS status VerifiedGood`.
`VerifiedBad`, malformed output, a contradictory known status, or an
unrecognized checker result is rejected.

External commands are JSON arrays and run without a shell. The placeholders
`{problem}`, `{solution}`, and `{artifact}` expand to absolute paths. For
example, with a separately installed ProofCheck 1.0:

```text
python3 tools/validation/validate_tptp_solution.py \
  problem.p solution.s \
  --proof-command-json \
  '["/opt/proofcheck/proofcheck","-p","{problem}","{artifact}"]'
```

ProofCheck 1.0 reports `Unknown` when a refutation depends on an
`introduced(definition)` step. For that first-order coverage class, the
repository provides an integrity-checking adapter for a caller-supplied
ProofGuard checkout:

```text
python3 tools/validation/validate_tptp_solution.py \
  problem.p solution.s \
  --proof-command-json \
  '["python3","tools/validation/run_pinned_proofguard.py",
    "--proofguard-root","/opt/proofguard",
    "--eprover","/opt/eprover",
    "--expected-eprover-sha256","<64-digit SHA-256>",
    "{problem}","{artifact}"]'
```

The adapter verifies the exact upstream Git remote, commit, clean worktree,
checker/engine hashes, and caller-declared E hash before running either
external program. It does not download or bundle ProofGuard. The pinned
ProofGuard revision has no upstream license declaration, so obtain any
required permission and keep its checkout outside Umlaut's source and runtime
packages.

The exit codes are:

| Code | Meaning |
| ---: | --- |
| 0 | Independently verified, or the final status makes no success claim |
| 1 | Rejected |
| 2 | Explicit coverage gap or inconclusive checker |
| 3 | Invalid input or checker configuration |

Use `--allow-coverage-gap` only for inventory runs that must continue and
inspect the JSON `verdict`; it does not turn a gap into verification.

Umlaut emits `CNFRefutation` objects for proof successes. Its explicit
finite-model worker emits complete `FiniteModel` interpretations, which can be
submitted to an independent semantic checker. Any success claim without its
required checkable artifact receives the explicit `coverage_gap` verdict. The
gate must not substitute a second solver's matching status for a proof or
model check.

## Bounded arithmetic and QE

`arithmetic_qe_oracle.py` is independent of Umlaut's arithmetic
implementation. It provides:

- exact rational, floor, and ceiling semantics through Python
  `fractions.Fraction`;
- complete bounded one-variable cell decomposition for rational affine terms
  with nested floors and ceilings;
- complete bounded integer enumeration;
- a shell-free external SMT-LIB process adapter;
- explicit `sat`, `unsat`, `unknown`, `disagreement`, and `error`
  classification; and
- a deterministic structural shrinker for preserving and minimizing
  disagreements.

The external solver is always caller supplied. No solver is linked, bundled,
or adopted as an Umlaut dependency. The reproducible pinned-Z3 experiment and
paper-errata mutation matrix are in
`experiments/2026-07-29-005-arithmetic-qe-oracle/`.
