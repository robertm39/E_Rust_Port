# Opt-in base VIRAS quantifier elimination

VIRAS quantifier elimination is an explicitly enabled arithmetic subsystem.
The `umlaut-viras-qe` executable remains the narrow standalone interface.
An all-feature `umlaut` build also exposes the nondefault
`--viras-qe-preprocess` mixed-problem path. Both surfaces are absent from the
default Cargo feature graph and CASC runtime package, and no automatic
schedule invokes either one.

The implementation is a clean-room Rust port of the paper-derived boundary in
[`viras_docs/`](../viras_docs/README.md) and the frozen prototype in
[`experiments/2026-07-30-004-base-viras-qe-prototype/`](../experiments/2026-07-30-004-base-viras-qe-prototype/).
No source from the unlicensed VIRAS implementation was inspected, copied,
linked, built, or executed.

## Build and run

Build the optional executable on the mandatory Linux runner:

```text
cargo build --locked --release --features viras-qe --bin umlaut-viras-qe
```

It reads one document from a file or standard input and emits canonical JSON
by default:

```text
umlaut-viras-qe --json problem.p
umlaut-viras-qe --tff problem.p
umlaut-viras-qe --json -
```

Exit status 0 means every quantifier was eliminated. Exit status 2 means the
input was rejected or bounded elimination returned `Unknown`; it never means
that a partially generated formula is false. `--help` lists the step,
candidate, grid, grid-point, DNF-branch, formula-node, and rational-bit limits.

JSON output uses schema `umlaut-viras-qe-v1`. A successful record contains the
normalized imported formula, quantifier-free result, canonical TFF
re-embedding, typed-import trace, complete candidate origins, every
grid-flattening record, resource counts, and
`"replay_validated":true`. A resource failure contains no result formula.
TFF mode emits a transformed single-formula document only after success.

Build the primary prover with the same feature to expose conservative
mixed-problem preprocessing:

```text
cargo build --locked --release --features viras-qe --bin umlaut
umlaut --viras-qe-preprocess --tstp-format --proof-object=1 problem.p
```

The option is not recognized in a default build. It uses the kernel's fixed
default resource limits and prints one `% VIRAS QE preprocessing:` record
with formula, import, proof-check, Unknown, node, and branch counts when
ordinary output is enabled.

## Supported typed fragment

The standalone document gate accepts exactly one `tff` formula with role
`axiom` or `conjecture`. The mixed-problem path instead uses Umlaut's normal
TPTP parser, includes, roles, type declarations, and formula owners. It
considers each active closed typed formula independently only when that
formula contains a quantifier. All unrelated formulas and clauses remain in
the ordinary problem. Both paths then use the same arithmetic importer, which
accepts:

- closed quantification over `$int` and `$real`;
- ground `$int`, `$rat`, and `$real` numerals, including exact fractions,
  finite decimals, and decimal exponents;
- addition, subtraction, unary minus, rational scaling, division by a
  nonzero ground constant, floor, ceiling, and the safe coercions;
- equality, disequality, order comparisons, `$is_int`, and the accepted
  ground `$is_rat` cases; and
- negation, conjunction, disjunction, implication, reverse implication,
  equivalence, exclusive equivalence, NAND, and NOR.

Integer binders are translated to real binders plus the exact guard
`X = floor(X)`. Quantified rationals, real-to-rational coercion, rationality of
a real, nonlinear products, variable or zero divisors, unsupported rounding
families, uninterpreted arithmetic, mixed numeric sorts, free variables,
additional annotated formulas, includes, and non-TFF dialects fail closed
under stable rejection codes in the standalone gate. Those document-level
constructs remain legal around individually eligible formulas in the mixed
path; unsupported formula bodies pass through unchanged.

The kernel language is linear integer/real arithmetic over arbitrary-precision
exact rationals, addition, rational scaling, and nested floor. Innermost
existential bodies are converted to bounded DNF and each conjunction is
eliminated by complete finite virtual substitution. Universals use exact NNF
duality. Every Boolean branch shares the caller's resource budgets.

## Derivation and trust boundary

For each successful conjunction, the kernel regenerates the complete
candidate set in a fresh kernel, replays every virtual substitution, and
compares the result and grid trace. Before the primary prover inserts a
result, a second formula-level native check re-runs bounded elimination from
the imported source and compares the full result, resource accounting, and
all branch derivations. The checked result is then rendered as canonical TFF,
parsed back through Umlaut's typed term bank, re-imported, and compared again.
Source, result, replay-flag, and candidate corruptions all fail before a
replacement term is returned.

Ordinary proof documentation first publishes and archives the original
source formula. The transformed active copy receives a no-argument
`DC_VIRAS_QE` derivation step, and TSTP/PCL documentation names the unary
`viras_qe` inference with `status(thm)`. Final proof extraction therefore
retains the original input leaf and nests the arithmetic step above it.
Import rejection and kernel `Unknown` do not alter the formula, properties,
role, identity, or ancestry. A native check or typed round-trip failure aborts
the prover rather than inserting an unchecked result.

This is a native checked Umlaut proof-publication rule, not a claim that
generic external TSTP checkers understand VIRAS arithmetic. The path remains
explicit and schedule-independent pending a separate adoption study.

## Exact arithmetic and package disablement

The `viras-qe` feature enables the pure-Rust `num-bigint`, `num-integer`,
`num-rational`, and `num-traits` crates and their `autocfg` build dependency.
Exact versions, checksums, license texts, transitive edges, and package
boundaries are recorded in
[`docs/dependency-packaging-matrix.md`](dependency-packaging-matrix.md).

Omitting `--features viras-qe` removes the entire arithmetic module and causes
Cargo to omit the feature-required `umlaut-viras-qe` target. The default
runtime links no optional crate code and retains its dependency-free feature
closure.

## Source map

- `src/arithmetic/viras.rs`: exact AST, profiles, grids, virtual terms,
  V1/V2/V3, bounded wrapper, derivations, replay, and tests.
- `src/arithmetic/typed_lira.rs`: typed Umlaut-AST importer, stable rejection
  taxonomy, and canonical TFF renderer.
- `src/arithmetic/viras_preprocess.rs`: formula-level native recheck, typed
  round-trip, pass-through outcomes, and corruption tests.
- `src/simple_apps/viras_qe.rs`: bounded CLI and canonical JSON/TFF output.
- `src/bin/umlaut-viras-qe.rs`: feature-required executable entry point.
- `src/prover/umlaut.rs`: explicit mixed-problem integration, counters, and
  source-ancestry-preserving proof documentation.
