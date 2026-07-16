# Formula-owner route differential corpus

## Question

Which C-accepted FOF/TFF/TCF/THF formula-owner shapes still fail or diverge in the Rust executable after the represented-owner parser routing work?

## Setup

The corpus was created on 2026-07-15 (America/New_York). The first-order corpus covers nested connectives and quantified TCF clauses. The LFHOL corpus covers applied lambdas, lambda-valued arguments, function equality, predicate quantification, choice, and description. The C reference is upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`; the LFHOL cases use its `--enable-ho` build.

```powershell
.\e-interop.ps1 compare `
  -Corpus .\experiments\2026-07-15-006-formula-owner-route-corpus\corpus-fol `
  -RustExe .\target\release\eprover.exe `
  -TimeoutSeconds 20

.\e-interop.ps1 compare `
  -Corpus .\experiments\2026-07-15-006-formula-owner-route-corpus\corpus-lfhol `
  -RustExe .\target\release\eprover.exe `
  -TimeoutSeconds 20
```

The executable comparison uses auto mode, deterministic rewrite/new-clause sorting, proof-object output, a 20-second CPU limit, and a 2 GiB memory limit.

## Results

The final first-order comparison is `.artifacts/e-compare/20260715-213404-700522/comparison.json`: all three cases are exact, including normalized proof output.

The final higher-order comparison is `.artifacts/e-compare/20260715-213404-700587/comparison.json`. All six cases have the same exit code and `Theorem` status. Choice, description, and higher-order predicate quantification have exact normalized proof output. The three lambda-bearing cases differ only in normalized proof output:

- C prints the simplified source as `plain`, while Rust retains its source `axiom` role at one intermediate proof node.
- Some C lambda-lift steps retain explicit Boolean `= $true` wrappers that Rust has already simplified away.

These are proof/CNF documentation differences, not parsing, ownership, status, or result differences. Broader mixed formula/clause proof extraction and byte comparison is already tracked by `E_Rust_Port-j76.1.12`.

Together with the exhaustive represented-owner route unit tests, the corpus found no C-accepted formula body that still requires the temporary simple-formula bridge. The remaining bridge cases are TCF clause ownership or C syntax/type boundaries.

## Falsification checks

The initial corpus incorrectly assumed that three Rust-supported FOF FOOL/formula-equality extensions were accepted by this C reference. Artifacts `.artifacts/e-compare/20260715-212718-408728/comparison.json` and `.artifacts/e-compare/20260715-213026-100976/comparison.json` show C exit 3 while Rust proves them:

- C's first-order parser rejects a formula-valued parenthesized right operand of `p(a) = (...)` where Rust's represented parser accepts Boolean equality.
- C `ParseIte` calls `TFormulaTSTPParse` for individual-valued branches and rejects the comma after the first individual branch.
- The corresponding first-order term-valued `$let` spelling is also rejected by C's formula parser.

Those inputs are retained as non-enumerated `.txt` fixtures under `rejected-by-c/`; they document Rust extensions and are not evidence of missing C behavior.

The first LFHOL run omitted parentheses around application-valued lambda bodies. C consequently parsed the trailing application variable outside the lambda scope and reported a free variable. Parenthesizing the lambda bodies made both executables accept the formulas and return `Theorem`, demonstrating that the original failure was precedence-sensitive corpus syntax rather than a missing owner route.

## Conclusion and limits

The broad Formula Sets parser item has no remaining demonstrated C feature gap. FOF/TFF bodies accepted by the represented term-formula parser are selected by a detached-bank probe; TCF bodies use the represented typed-clause parser when they are clauses; THF bodies always use represented ownership. The previously named bridge-only TCF `$distinct`, non-clausal TCF, and non-Boolean top-level THF cases are C boundaries documented in `../2026-07-15-005-formula-owner-boundaries/FINDINGS.md`. The exact lambda-lift `PDTree` and helper-comparison portions of the same migrated item are also resolved there.

This experiment does not claim byte-identical higher-order proof objects. The three lambda proof-documentation mismatches remain under the dedicated proof-extraction task rather than the formula-owner parser task. It also does not require Rust to reject valid extensions that the C 3.3.5 parser happens not to accept.

## Validation

The two final differential runs enumerated nine cases. All nine matched exit code and theorem status; six were normalized-output exact. These focused Rust route regressions also passed:

```powershell
cargo test --all-features tstp_fof_tff_formula_owner_route_uses_represented_parser_probe
cargo test --all-features tstp_tcf_formula_owner_route_uses_represented_clause_parser_probe
cargo test --all-features app_encode_tstp_entries_route_non_distinct_formulas_to_represented_owners
```
