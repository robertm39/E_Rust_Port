# eground give-up estimate parity

## Status

Completed for Bead `E_Rust_Port-j76.2.118`. Rust now preserves the distinct C
estimate behaviors for unconstrained and constrained grounding while keeping
the low-level process exit represented as an explicit library outcome. The
vendored C source remained unchanged.

## Fresh mismatch

The archived-C/Rust matrix initially showed Rust stopping on this unconstrained
case while C emitted all four ground units:

```text
eground --lop-in --silent --give-up=1
p(a).
p(b).
q(X).
```

Source inspection alone suggests `2^1 > 1`, but a symbolized diagnostic build
in the managed WSL cache exposed the actual runtime values at
`ccl_grounding.c:1024`: `give_up == 1`, `vars == 1`, and local `tmp == true`.
The declaration is `bool res = true, tmp;`, so assigning
`PStackGetSP(default_terms)` truncates the two constants to Boolean one. Every
nonempty unconstrained default-term stack consequently estimates `1^vars`.

## Resolution

Rust's unconstrained estimate helper now performs the same Boolean truncation.
The constrained helper is unchanged: C stores `varinstestimate(inst)` in a
`double` and compares the real running member-plus-clause estimate before
generation. Rust still returns `EstimateLimitExceeded` to reusable callers and
the executable writes the C failure line, flushes, and returns success before
normal result/statistics/resource output.

The permanent matrix adds a constrained `--give-up=1` case beside the existing
unconstrained case. Both are exact against archived upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The 22-case report is:

`.artifacts/e-compare/20260717-015547-388865-tools/`

It has four remaining diagnostic-only mismatches; both give-up cases, the
DIMACS split-output case, and all explicit propositional output routes match.

## Validation

- 37 focused `clauses::grounding` tests pass;
- 30 focused `prover::eground` tests pass serially;
- 33 comparison-harness tests pass;
- optimized `eground` build passes;
- both permanent give-up cases are exact against archived C.
