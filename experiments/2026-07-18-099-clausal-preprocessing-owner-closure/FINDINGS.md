# Clausal Preprocessing Owner Closure

## Status

Completed for Bead `E_Rust_Port-j76.2.39`. The migrated umbrella gap was
superseded by narrower equality-definition, goal-definition,
predicate-elimination, BCE, and higher-order option-effect slices. A fresh
combined audit and rerun of every retained C/Rust reference found no remaining
formula-owner preprocessing path to port. The vendored C checkout remained
unchanged.

## Compatibility decision

Unchanged C calls `FormulaSetCNF2` before `ProofStateClausalPreproc`. The later
equality-definition unfolding, BCE, predicate elimination, and goal-definition
operations accept clause sets only; C exposes no parallel formula-set API for
any of them. Rust follows the same owner transition: represented formula owners
are drained through CNF before the clause preprocessing pipeline.

C and Rust both run BCE and predicate elimination only when the syntactic
process problem type is first-order. A THF input therefore skips both passes
even when its lowered clauses are first-order-shaped. Goal-definition
transformation is not under that gate. Presaturation interreduction remains a
later proof-control operation after initial clause documentation and
initialization.

## Combined owner audit

[`audit_preprocessing_owners.py`](audit_preprocessing_owners.py) checks 19
source, ordering, gate, regression, and retained-reference facts:

- formula CNF precedes clause preprocessing in both implementations;
- the C and Rust clause-pass orders agree;
- the BCE and predicate-elimination first-order gates agree;
- the checked C headers expose no formula-set variant of the four clause-only
  transformations;
- permanent formula-origin regressions cover BCE, predicate elimination, goal
  definitions, equality-definition unfolding, and presaturation;
- a permanent THF regression pins the first-order-only pass gate; and
- all five narrower retained comparisons remain exact.

The retained [`owner-audit.json`](owner-audit.json) passes 19/19 checks.

## Fresh reference reruns

The current release `eprover.exe` SHA-256 was
`E4CAB1204C7F57AA50BDA1CD71FE869FFB4FC466A01506311A95C51E7F488A69`;
the current release `classify_problem.exe` SHA-256 was
`3A275E6FFE1DF31ED421F314F2C4E001BD6D80FD9FBBA325EBC14BDAE3C3F299`.
Fresh runs reproduced every retained reference byte for byte:

| Surface | Evidence | SHA-256 |
| --- | --- | --- |
| formula-origin equality-definition boundary | 2/2 exact classifier cases and 10/10 owner checks | `D59575E0FF02703C48C83CDEB83C5883B23C0D5FECC3AC1DA05B94D072DB836D` |
| formula-origin goal definitions | 4/4 exact traces and 14/14 owner checks | `EE9345869F907B24D35651FDCC2A90AE58C49A32A25158CC2F9D3510391FD97C` |
| predicate elimination | unchanged C, internal SAT, and runtime PicoSAT exact | `05B586ADD2DE7EB5C768805F85DF057D32157A7DF61F9E9947BD3193E51960FA` |
| BCE | exact progress, clauses, statistics, status, and exit | `00CF45889533B82976A050FDF13BD309734A325AA360353F0CC00BA72230FD4E` |
| FO/THF option effects | 15/15 exact cases and 72/72 route checks | semantic report `7ccc8251a6ff33206ca58707ec9e39fe98da6cdfd03a4ca431489add210d4b26` |

The isolated C references use unchanged upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --bin classify_problem --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-099-clausal-preprocessing-owner-closure\audit_preprocessing_owners.py `
  --output target\preprocessing-owner-audit.json
```

Rerun the comparison commands documented in experiments `076`, `077`, `078`,
`079`, and `094` with their retained `--expected` files to reproduce the five
reference checks summarized above.
