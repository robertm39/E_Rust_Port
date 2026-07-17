# Low-level term/formula parser reconciliation

## Objective

Reconcile `E_Rust_Port-j76.2.103` against the current Rust implementation and the immutable proof-rendering boundary completed in experiment 033. This is an API, caller, and ownership audit; the vendored C source remains unchanged.

## Surface audit

| C surface | Rust surface | Production evidence |
| --- | --- | --- |
| `FuncSymbParse`, `TermParseOperator`, `TermSigInsert` | `func_symb_parse`, `term_parse_operator`, `term_sig_insert` | signature/type parsing plus unshared and banked term parsers |
| `TermParse`, `TermParseArgList` | `term_parse` and recursive argument/list parsing | learning, replacement, and direct term tools |
| `TBTermParseSimple`, `TBTermParseReal` | `TermBank::parse_term_simple`, `parse_term_with_distinct_checks` | clause/equation parsing and helper term targets |
| application half of `TFormulaTSTPParse` | `TermBank::parse_tstp_application_term` | executable higher-order term targets, including `enormalizer` |
| `TFormulaTPTPParse`, `TFormulaTSTPParse`, `TcfTSTPParse`, `TSTPDistinctParse` | public term-bank/clause helpers | represented eprover, batch, classifier, normalizer, and patternizer formula owners |
| `TBStorage` | `TermBank::storage_estimate` | proof-state cleanup-limit accounting |
| scanner file/include frames | `Scanner::from_file`, `parse_include`, executable selector stacks | proof search, printing, helper CNF, repeated and nested includes |

Direct regressions cover identifier classification, signature property insertion, uppercase applied heads, list literals, distinct-symbol diagnostics, Boolean and non-Boolean `$ite`/`$let` terms, scoped variables, fresh-atom expected-sort recovery, TPTP/TSTP operator and quantifier behavior, higher-order applications, `$distinct` constant-only parsing, TCF prefixes, storage accounting, and nested include-selector ownership.

## Remaining-boundary classification

The migrated issue's old "full formula-owner integration" and "scanner file/include handling" limitations are stale: represented formula owners and nested selector-aware include frames are production paths. The remaining term-valued FOOL atom ambiguity is not a missing exported parser. It is the compatibility-visible consequence of C encoding terms and formulas in one term grammar and is already isolated in the `ccl_tformulae` atom/`$ite`/`$let` reviews. The checked-versus-simple parser split is likewise retained and tracked by the `cte_termbanks` post-compatibility review.

C `TFormulaTPTPPrint` still reaches `EqnAlloc` and can classify predicates while printing. Rust now makes the rendering boundary immutable: it preserves C's text artifacts through a print-only literal view but does not mutate the signature or shared terms. Experiment 033's regression renders formula-backed and clause-backed proof nodes and verifies every stored-term property, canonical lookup identity, and term-bank counter is unchanged. The paired stale `ccl_tformulae` review items are resolved by that evidence.

## Validation

- 216 distinct focused parser, include, accounting, and immutable-rendering tests passed.
- This documentation/tracking-only reconciliation retains the exact runtime baseline from commit `ae2e0762`: 4,233 default-feature library tests; 4,238 all-feature library tests; all binary targets; 7 integration tests; strict all-target, all-feature pedantic Clippy; and a release `eprover` build.
- `cargo fmt --all -- --check` passed.
- C-source documentation coverage, Change Later wording, Markdown-link integrity, and regeneration-preservation gates passed.
- The vendored `eprover/` worktree remained clean.
