# Formula Proof-Copy Ancestry

Date: 2026-07-09

## Hypothesis

Rust omitted intermediate formula nodes from proof objects because `FormulaSetCNF2` archived flat copies without the C `DCFofQuote` edge, while formula-parent lookup used only the visible id that `WFormulaFlatCopy` deliberately duplicates.

## Setup

- C references: archived normalized `ans_test06.p` and `socrates.p` outputs in `.artifacts/e-compare/20260709-224129-729562/`.
- Rust candidate: debug and release `eprover` on the bundled smoke-test files with `--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1`.
- C source reviewed: `FormulaSetCNF2`, `WFormulaFlatCopy`, `DerivationStackPCLPrint`, `DerivationStackTSTPPrint`, `DerivationTopoSort`, and `DerivationRenumber`.

## Findings

- C pushes `DCFofQuote` from the final CNF working copy to the extracted pre-CNF wrapper. Rust recorded the source as result metadata but omitted the derivation entry in both CNF drains.
- `WFormulaFlatCopy` copies the visible formula id. C derivations retain raw wrapper pointers until graph extraction and renumbering, while Rust's id-only formula refs collapsed distinct archived copies.
- C renders both `DCCnfQuote` and `DCFofQuote` as direct parents inside derivation expressions. Rust special-cased only the clause quote and emitted `inference(QUOTE,...)` for formula copies.
- C starts proof renumbering at one plus the largest numeric input-formula counter, not at one. Named input formulas therefore begin the internal sequence at zero even though their source names remain visible.
- The represented proof walk had one extra parent-order reversal relative to C's stack-pop traversal.
- Silent CNF formula modifications still call C `DocFormulaModification`, which clears `CPInputFormula` below the output gate. Rust preserved that property and printed transformed axioms with role `axiom` instead of `plain`.

## Result

- Formula refs now include a stable internal wrapper-source key while retaining the C-visible id.
- Both CNF drains restore the missing `DCFofQuote`; source-less helper refs retain numeric-id fallback.
- Formula quote rendering, input-name resolution, proof-id start, direct-parent order, and the silent input-property side effect now match C.
- `ans_test06.p` and `socrates.p` proof output match the archived C output after replacing the concrete problem path.
- The release executable matches the complete archived normalized output for both cases, modulo trailing-newline normalization.
- `ALL_RULES.p` still matches C's preprocessing class, selected preprocessing strategy, and `Theorem` result, but its normalized proof remains different in variable numbering, input ordering, and later ancestry.
- Focused formula-set, derivation, proof-state, proof-object, and exact answer-proof regressions pass. The complete all-target/all-feature suite passes 3,984 library tests and 3 scheduler integration tests.

## Limits

- The C executable was unavailable for a fresh full 50-case comparison, so only archived reference outputs were used.
- Multi-root proof lists and derivation shapes not exercised by these smoke tests still need broader reference comparison; `ALL_RULES.p` is the next concrete proof-ordering case.
