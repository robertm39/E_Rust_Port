# Pre-CNF Input Ownership and Auto Classification

Date: 2026-07-09

## Hypothesis

Rust selected the wrong automatic preprocessing strategy for `ALL_RULES.p` because it stored parsed `cnf(...)` records directly as clauses before raw feature classification, while C stores every non-watchlist input as a formula wrapper until CNF.

## Setup

- C reference: archived normalized output in `.artifacts/e-compare/20260709-224129-729562/mismatches/0001/reference.normalized`.
- Rust candidate: release `eprover.exe` on `eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p`.
- Source review: `FormulaAndClauseSetParse` in `eprover/CLAUSES/ccl_formulafunc.c` and `RawSpecFeaturesCompute` in `eprover/HEURISTICS/che_rawspecfeatures.c`.

## Commands

```powershell
target\release\classify_problem.exe --raw-class --tstp-format eprover\EXAMPLE_PROBLEMS\SMOKETEST\ALL_RULES.p
target\release\eprover.exe --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\SMOKETEST\ALL_RULES.p
cargo test --test eprover_schedule auto_mode_classifies_cnf_inputs_as_pre_cnf_formula_owners
```

## Results

- Before the fix, Rust classified the problem as `FSSSSMSSSSSNFFN` and selected `G-E--_302_C18_F1_URBAN_RG_S04BN`; C classified it as `FSMSSMSSSSSNFFN` and selected `G-E--_208_C18_F1_SE_CS_SOS_SP_PS_S5PRR_RG_S04AN`.
- The direct Rust clause set contributed standard weight 77 and the two formula owners contributed 99, for 176, below the medium-term threshold of 200.
- C parses all eight ordinary clauses as clause-backed formulas. Their explicit equality and disjunction cells add 44 weight units, producing 220 and crossing the threshold. The Rust `classify_problem` formula-only destination already exposed the same 220 total.
- Routing main-executable inputs through the existing formula-owner destination restores C's class and both selected strategy names. The theorem still proves.
- Keeping the formula owner also restores C's separate initial-formula and `DCFofQuote` clause nodes. Formula proof documentation must render those clause-backed owners as PCL/TSTP clause cores rather than closed FOF bodies; the proof-list and graph paths now do so.
- A 15-second `LUSK6.lop` run still reaches `ResourceOut`; no same-limit C measurement was available, so this is a narrow Rust runtime observation rather than a demonstrated C/Rust performance mismatch.

## Conclusion

Pre-CNF ownership is observable strategy semantics, not just an internal representation choice. Non-watchlist clause syntax must remain formula-backed until raw classification and `FormulaSetCNF2` have run.

## Limits

- The C executable could not be rerun in the current environment; comparison used the archived reference output and the reviewed C source.
- `ALL_RULES.p` proof ancestry and normalized proof text still differ after the strategy header aligns.
