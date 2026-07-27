# Higher-order option-effect reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.43`. All 22 migrated option routes pass
the static consumer audit, and all 15 focused executable cases are exact
against the isolated higher-order C build after three compatibility fixes. The
vendored C checkout remained unchanged.

## Question

Do the migrated higher-order executable options reach their production
consumers, and do their observable parsing, strategy-printing, and
preprocessing effects match C?

## Static routing audit

[`audit_option_effects.py`](audit_option_effects.py) checks the CLI spelling and
the production bridge/consumer for:

- extensional-superposition depth, inverse recognition, injectivity-definition
  replacement, BCE, and predicate elimination;
- lambda-to-forall, eta normalization, higher-order ordering, Leibniz-equality
  elimination, and formula-only unrolling;
- primitive enumeration mode/depth, choice-recognition depth, local rewriting,
  argument pruning, and induction preinstantiation; and
- functional projection, unification mode, both unification oracles, and the
  unifier/step limits.

It also checks the exact C and Rust first-order gates for BCE and predicate
elimination and the Rust strategy-print prefixes for those passes. The retained
report passes 72/72 checks across 22 routes and has SHA-256
`0ba034bf3fb16bdf89ea36499211ef5f15ed16efd6fe6d9775e08aeb3aba0f6c`.

## Executable matrix

[`compare_option_effects.py`](compare_option_effects.py) compares the Windows
Rust release executable with the isolated `--enable-ho` C build from upstream
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The C executable SHA-256 is
`317e261b4915d16834de9f5a133ecd07fe6e21dfdc8c5f06072ed75b3e56b7e1`;
the compared Rust executable SHA-256 is
`30dd2f99707ee5bc9d8f5a74fba09b1b26a5d4887916311b0202162cad36b360`.

All 15 cases match:

| Case group | Cases | Result |
| --- | ---: | :---: |
| current strategy with all 22 overrides | 1 | exact |
| BCE/predicate-elimination strategy prefixes | 2 | exact |
| invalid eta/order/enumeration/unification/depth/limit values | 6 | exact |
| baseline and BCE/predicate-elimination combinations on THF input | 4 | exact |
| observable BCE and predicate-elimination effects on FOF input | 2 | exact |

Raw option and strategy cases compare exit status, stdout, and stderr byte for
byte. Proof preprocessing cases compare stderr, BCE/PE lines, final clauses,
and four preprocessing counters. Generated clause identifiers are normalized
with `c_0_-?\d+`: C's predicate-elimination result uses negative parent IDs
where Rust uses ordinary positive compact IDs, but the clauses and derivation
shape otherwise match. Terminal proof-search status is excluded from this
projection because the independent exhausted-axioms incompatibility is already
tracked by `E_Rust_Port-j76.2.140`.

The retained comparison report has SHA-256
`7ccc8251a6ff33206ca58707ec9e39fe98da6cdfd03a4ca431489add210d4b26`.

## Compatibility fixes

### Syntactic first-order gate

C runs BCE and predicate elimination only when the process-global
`problemType == PROBLEM_FO`. It therefore skips both passes for a THF file even
when clausification happens to produce first-order-shaped clauses. Rust had
instead inspected that lowered clause surface and ran the passes. The
executable now uses the same syntactic problem-type gate; all four THF variants
are identical to their baseline, while the FOF fixtures still demonstrate
both preprocessing effects.

### Strategy-print preprocessing output

C reaches `ProofStateClausalPreproc` before `strategy_io`, so
`--print-strategy` with BCE or predicate elimination enabled prints the empty
pass summaries before the strategy cell. Rust now preserves the exact lines,
punctuation, and ordering after the unconditional preprocessing-configuration
line.

### Invalid unification mode exit

C reports an invalid `--unif-mode` value through `Error(..., 0)`. The diagnostic
is therefore printed on stderr but the process exits successfully. Rust had
used the normal usage-error exit. It now preserves C's misspelled diagnostic
and exit status 0.

## Residual scope

This result proves option materialization and the focused observable effects;
it does not claim completion of the general higher-order formula/CNF bridge.
Unsupported higher-order formula shapes and remaining clausification ownership
are still tracked by the narrower Beads `E_Rust_Port-j76.2.42` and
`E_Rust_Port-j76.2.41`.

## Validation

- static option/consumer audit: 72/72 checks across 22 routes;
- focused executable comparison: 15/15 exact cases;
- permanent Rust regressions for the FO-only gate and strategy prefixes;
- retained-reference reruns for both experiment scripts;
- full all-target/all-feature Rust suite and strict pedantic Clippy;
- release `eprover` build and all C-source documentation integrity gates; and
- clean nested `eprover/` worktree.
