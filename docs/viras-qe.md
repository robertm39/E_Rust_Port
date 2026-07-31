# Opt-in base VIRAS quantifier elimination

`umlaut-viras-qe` is a standalone, explicitly enabled arithmetic
quantifier-elimination tool. It is not called by `umlaut`, is absent from the
default Cargo feature graph and CASC runtime package, and has not changed any
automatic schedule.

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

## Supported typed fragment

The document gate accepts exactly one `tff` formula with role `axiom` or
`conjecture`. Umlaut's ordinary scanner, typed term parser, signature, and
arithmetic symbol typing run first. The arithmetic importer then accepts:

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
under stable rejection codes.

The kernel language is linear integer/real arithmetic over arbitrary-precision
exact rationals, addition, rational scaling, and nested floor. Innermost
existential bodies are converted to bounded DNF and each conjunction is
eliminated by complete finite virtual substitution. Universals use exact NNF
duality. Every Boolean branch shares the caller's resource budgets.

## Derivation and trust boundary

For each successful conjunction, the production wrapper regenerates the
complete candidate set in a fresh kernel, replays every virtual substitution,
and compares the result and grid trace before publishing it. Focused
corruption tests remove candidates and alter result formulas; replay rejects
both. The held-out production experiment independently checks successful
closed results against exact rational evaluation and an external reference
outcome.

This derivation is an auditable standalone transformation record, not a TSTP
proof accepted by Umlaut's first-order proof publisher. Until a native proof
rule/checker covers arithmetic transformations, the primary prover must not
silently insert these results into a refutation. This is why the path remains
standalone and schedule-independent.

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
- `src/simple_apps/viras_qe.rs`: bounded CLI and canonical JSON/TFF output.
- `src/bin/umlaut-viras-qe.rs`: feature-required executable entry point.
