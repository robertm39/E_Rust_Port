# Formula-owner convergence

## Status

Completed for the final 19 summarized post-compatibility records under Beads
`E_Rust_Port-j76.3`. The records described one observable formula pipeline
through several historical parser bridges and future-owner notes. Production
input now converges on `ProofState::f_axioms`/`FormulaSet`, and every named
consumer is represented. The remaining simple-parser and app-encode
lookaheads are retained compatibility implementation details rather than a
demonstrated missing C feature. The vendored C checkout remains unchanged.

## Production ownership boundary

[`audit_formula_owner_convergence.py`](audit_formula_owner_convergence.py)
passes all 43 current C-source, Rust-source, and retained-evidence checks.
[`audit-reference.json`](audit-reference.json) pins report SHA-256
`34202ae6751fbf6a5bb04c3a2f78a29c97d6a8f3ca013b278970c7cfa38d542b`.

The important production invariant is narrower and stronger than the old
bridge wording:

- `InputOwnerDestination` exposes only print and CNF `FormulaSet`
  destinations in production; its clause destination and `ClauseBridge` mode
  are compiled only for tests.
- Old TPTP formulas use `TermBank::parse_tformula_tptp`; represented
  FOF/TFF/THF formulas use `TermBank::parse_tformula_tstp`; and represented TCF
  clauses use `tcf_tstp_parse`.
- Directly represented bodies preserve role, source, question annotation,
  typed binders, declared application heads, Boolean-vs-term equality, formula
  application, FOOL `$ite`/`$let`, and free-variable checks in typed term
  formulas.
- A compatible body handled by the simple fallback still becomes a
  `WrappedFormula` in the same production `FormulaSet`. CNF input clauses are
  likewise wrapped as formulas. The fallback does not create a second
  production owner pipeline.
- Formula owners pass through aggregate problem-type restoration,
  `$distinct`, raw classification, threshold/GSInE/definition selection,
  conjecture preprocessing, the complete ordered formula-set CNF stages,
  proof-state initialization, typed printing, and app encoding.

The route split remains useful because C has dialect-specific rejection
boundaries. Removing it solely for structural similarity would add risk and
would not improve the drop-in surface.

## Fresh differential route corpus

The original nine-case formula-owner corpus was rerun on Ubuntu 24.04 against
fresh unchanged-C FOL and `--enable-ho` builds. [`route-reference.json`](route-reference.json)
pins the compact result and both C executable hashes.

All nine cases have identical C/Rust exit codes and statuses. Six are
normalized-output exact:

- all three FOL cases, including nested connectives and quantified TCF;
- quantified THF predicate application; and
- the two choice/description fixtures, which both current executables reject
  at the same boundary with exit 3 and no status.

The applied-lambda, function-equality, and lambda-argument cases all prove
`Theorem` in both executables and retain only the known proof-document
presentation differences. Those differences are not parser, owner, CNF
structure, status, or result gaps and are covered by the completed main
compatibility matrix.

## Summarized-record decisions

The final records resolve as four evidence groups:

| Records | Decision |
| --- | --- |
| `54`, `55`, `101`, `331`, `427`, `502` | Syntax-only, old/TSTP formula input, SInE, term/formula-bank ownership, definition selection, and pre-CNF raw classification all consume the represented production owner pipeline. |
| `56`–`59`, `61`–`63`, `539`, `540`, `552`, `553` | Application heads, typed binders, FOOL, equality, Boolean/user-`bool`, let scope, ITE/FOOL pass order, and formula application have typed term-formula implementations and exact route evidence. Narrow lookahead remains a compatible dispatch policy. |
| `90` | App encoding owns a `FormulaSet`, preloads types, preserves include echoes and stdout side-channel order, and has exact typed-application evidence. |
| `649` | The parser, owner, type-printing, and executable-output parts converge above. The no-per-term-cache LFHO design is already resolved by Experiment 336. |

These are compatibility/design closures, not claims that Rust must copy C
pointer layout or parser function boundaries.

## Retained narrow work

Closing summarized record `649` does not close the two independent detailed
`cte_termbanks` reviews:

- `E_Rust_Port-j76.4.1277` retains the exact `SubstNormTerm` WHNF question.
- `E_Rust_Port-j76.4.1288` retains the exact
  `TBInsertOpt(DEREF_ALWAYS)` WHNF question.

They remain explicit detailed backlog owners and will be reconciled with the
rest of `E_Rust_Port-j76.4`.

## Validation

- current formula-owner convergence audit: 43/43;
- fresh C/Rust route corpus: 9/9 exit and status matches, 6/9 normalized-output
  exact, three known proof-document-only differences;
- retained formula-owner mode matrix: 28/28 byte-exact;
- retained raw-spec comparison: 2/2 byte-exact;
- retained SInE owner comparison: exact;
- retained formula/CNF pipeline evidence: 69 static checks and five LFHOL CNF
  projections;
- retained explicit-bank/no-per-term-cache audit: 15/15; and
- final Rust snapshot validation from Experiment 337: formatting, strict
  all-target/all-feature Clippy, release build, and 4,425 tests.

The audit can be rerun locally because it only reads source and retained
artifacts:

```powershell
python experiments/2026-07-25-037-formula-owner-convergence/audit_formula_owner_convergence.py `
  --repo . `
  --expected experiments/2026-07-25-037-formula-owner-convergence/audit-reference.json
```
