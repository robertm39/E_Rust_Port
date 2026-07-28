# Independent solution validation gate

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

The exit codes are:

| Code | Meaning |
| ---: | --- |
| 0 | Independently verified, or the final status makes no success claim |
| 1 | Rejected |
| 2 | Explicit coverage gap or inconclusive checker |
| 3 | Invalid input or checker configuration |

Use `--allow-coverage-gap` only for inventory runs that must continue and
inspect the JSON `verdict`; it does not turn a gap into verification.

Umlaut currently emits `CNFRefutation` objects for proof successes. Complete
saturation can justify `Satisfiable` or `CounterSatisfiable`, but Umlaut does
not yet emit a TPTP finite interpretation. Those claims therefore receive the
explicit `coverage_gap` verdict. The gate must not substitute a second solver's
matching status for a checkable model.
