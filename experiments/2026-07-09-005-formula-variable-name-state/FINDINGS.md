# Formula Variable-Name State

Date: 2026-07-09

## Question

Why did Rust print the `ALL_RULES.p` conjecture binders as `[X1, X2, X3, X4, X5]` while C printed `[X3, X4, X1, X2, X5]`, and did that difference account for the remaining proof mismatch?

## Setup

- C reference: `.artifacts/e-compare/20260709-224129-729562/mismatches/0001/reference.normalized`.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p`.
- Rust release command:

```powershell
target\release\eprover.exe --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\SMOKETEST\ALL_RULES.p
```

- C source reviewed: `ClauseParse`, `WFormulaParse`, `VarBankClearExtNames`, `VarBankExtNameAssertAlloc`, and quantified formula parsing.

## Findings

- C defaults `ClausesHaveLocalVariables` to true, so each clause clears the external-name map and resets per-sort variable allocation before parsing.
- `WFormulaParse` does not apply that local-clause reset. It clears names only for the separate disjoint-variable mode.
- The final clause before the FOF records encounters source names in the order `X3, X4, X1, X2`. Its printed variable codes are therefore `X1, X2, X3, X4`, while those source-name bindings remain in the shared bank.
- The following conjecture requests binders in source order `X1, X2, X3, X4, X5` and consequently receives printed codes `X3, X4, X1, X2, X5`, exactly matching C.
- Rust cleared external names unconditionally at every formula entry in the main executable, app-encode, batch, and `enormalizer` paths.

## Falsification Checks

- A mixed CNF/FOF regression constructs the same source-name permutation and asserts the exact inherited binder and body variable codes.
- All 3,988 library tests and all three schedule integration tests pass after removing the formula-only resets; the focused `eprover`, batch, and `enormalizer` suites also pass independently.
- `ALL_RULES.p` retains C's preprocessing class, selected strategy, and `Theorem` result, and its input conjecture now matches C exactly.
- AC status output still reports `f` as AC and `g` as commutative, ruling out failed AC recognition as the cause of the remaining proof-path difference.

## Conclusion

Cross-record external-name state caused the variable-numbering mismatch. Preserving it fixes the earliest normalized divergence, but does not make the complete proof equal: Rust still finds a 31-line proof that uses associativity as an ordered rewrite, while the archived C proof has 49 lines and uses AC resolution plus additional axioms.

## Limits

- The archived C run does not include a clause-selection trace, so the later rewrite-eligibility or evaluation-order divergence is not yet isolated.
- The state coupling is preserved for compatibility, but should be replaced by an explicit parser scope mode after drop-in behavior is secured.
