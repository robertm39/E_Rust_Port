# Formula-pipeline scope reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.42`. The migrated umbrella mixed stale
implementation claims with parser-spelling and full-proof concerns that already
have narrower owners. The represented C-shaped formula/CNF pipeline, formula-
aware heuristic initialization, typed output, state-owned higher-order indexes,
and complete checked-C higher-order dispatcher are present. All five vendored
LFHOL examples reach CNF in both builds with matching focused projections. The
vendored C checkout remained unchanged.

## Question

Does this legacy item still identify an unimplemented formula transformation or
higher-order ownership branch, or has that work moved into represented
production owners and narrower compatibility tasks?

## Static ownership audit

[`audit_pipeline_scope.py`](audit_pipeline_scope.py) checks both the pinned C
source and the Rust production routes rather than relying on the migrated prose.
Its 69 checks cover:

- the executable calls to formula-set CNF, formula-aware proof-control
  initialization, and formula-set app encoding;
- every ordered `FormulaSetCNF2` phase: named-to-DB, ITE/LET lifting,
  definition-symbol unfolding, lambda normalization, FOOL unrolling,
  simplification, definition introduction, original/copy archival, wrapped CNF,
  post-CNF lambda lifting, and garbage collection;
- proof-state ownership and allocation of both ExtSup indexes, plus the
  production selected-clause consumer;
- all ten checked-C higher-order dispatcher effects: ArgCong, NegExt, C-gated
  PosExt, inverse recognition, ExtSup, ExtEqRes, ExtEqFact, Leibniz elimination,
  primitive enumeration, and choice instantiation;
- formula/signature-aware user WFCB/HCB installation;
- `--free-numbers`/`--free-objects` proof-state allocation; and
- represented THF formula ownership, typed clause/formula printing, and
  app-encode type preloading.

The retained report passes 69/69 checks and has SHA-256
`471d2e69ca4d6097091f454c1fd2487669a3c16e6147a7435224715ea47cccd4`.

## LFHOL CNF comparison

[`compare_lfhol_cnf.py`](compare_lfhol_cnf.py) compares the Windows Rust release
executable with the isolated `--enable-ho` C build from upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The C executable SHA-256 is
`317e261b4915d16834de9f5a133ecd07fe6e21dfdc8c5f06072ed75b3e56b7e1`;
the compared Rust executable SHA-256 is
`30dd2f99707ee5bc9d8f5a74fba09b1b26a5d4887916311b0202162cad36b360`.

The corpus is the complete vendored `eprover/EXAMPLE_PROBLEMS/LFHOL` directory:

| Fixture | Focus | Result |
| --- | --- | :---: |
| `lists.p` | applied functions, arrow variables, quantified equalities | exact |
| `permute_func_axioms.p` | existential higher-order function | exact |
| `permute_func_no_axioms.p` | existential higher-order function | exact |
| `SEV286^5.p` | quantified functional congruence | exact |
| `sledgehammer.p` | 82 formulas, lambdas, definitions, Skolemization | exact structural projection |

For every case the comparison pins exit status, stderr, generated type
declarations, the four initial/preprocessing counters, and final positive-unit,
negative-unit, and non-unit CNF sections. Generated clause/declaration IDs and
section order are normalized. The first four final clause sets then compare
exactly.

The large `sledgehammer.p` result has identical 40 type declarations, 82 parsed
axioms, 142 initial clauses, 48 preprocessing removals, and 94 saturation
clauses split as 12 positive units, two negative units, and 80 non-units. C and
Rust choose different quantified-variable and literal traversal orders inside
some of those clauses. Its retained projection therefore compares a per-clause
multiset of all non-variable tokens/operators, punctuation counts, and variable-
occurrence histograms. This ignores alpha-renaming and commutative clause
presentation while still detecting missing symbols, literals, quantifiers, or
term structure. All 94 shapes match.

The retained comparison report has SHA-256
`2852aa2189abf97ccc2fe433068485e679c7b959e7e1718bc75e795149d49387`.

## Scope decision

The old phrase "full CNF transformation" no longer describes a missing
`FormulaSetCNF2` implementation: the complete checked-C phase sequence is
represented and exercised by real LFHOL files. Likewise, higher-order indexes
are owned by `ProofState`, and there are no unported `ComputeHOInferences`
branches. Formula-aware user weight/heuristic definitions, free-symbol policy,
typed rendering, and app encoding already have production owners and permanent
tests.

This does not claim that every C-accepted surface spelling is byte-identical or
that full higher-order proof search is complete. Remaining bridge-only parser
spellings and their cleanup live in the existing narrow parser/formula-owner
reviews; the adjacent option-surface reconciliation remains
`E_Rust_Port-j76.2.41`; and the known full-proof `sledgehammer.p` mismatch stays
under its existing saturation/inference owners. Those concerns should not keep a
second umbrella implementation backlog open.

## Validation

- static C/Rust ownership and phase audit: 69/69 checks;
- vendored LFHOL CNF comparison: five/five focused projections;
- retained-reference reruns for both experiment scripts;
- full all-target/all-feature Rust suite and strict pedantic Clippy;
- release `eprover` build and all C-source documentation integrity gates; and
- clean nested `eprover/` worktree.
