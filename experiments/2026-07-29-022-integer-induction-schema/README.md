# Restricted integer-induction schema

This experiment evaluates one proof-explicit, lower-bounded integer-induction
schema for Bead `E_Rust_Port-9jt.6.3`.

Umlaut's existing `--preinstantiate-induction` option ports E's higher-order
preprocessing step. It instantiates clauses that already look like induction
axioms; it does not generate an induction principle. The prototype here is an
outcome-independent TPTP source transformation. It recognizes a narrow
integer conjecture, adds one named induction axiom, and leaves Umlaut's
production search code unchanged.

The schema is:

```text
( P(b)
  & ! [N: $int] :
      ( ( $greatereq(N,b) & P(N) )
     => P($sum(N,1)) ) )
=>
! [N: $int] :
  ( $greatereq(N,b) => P(N) )
```

The TPTP arithmetic semantics make this valid for every integer literal
`b`. The generator accepts only a single lower-bounded integer target with a
quantifier-free property. `schema.py` and `verify_schema.py` independently
enforce that contract. Generated proof claims are checked against the
augmented problem, so the induction axiom remains visible as a proof leaf.

Umlaut does not predeclare the standard types of the arithmetic operators.
`prepare.py` therefore adds redundant TPTP type declarations for the fixed
integer symbols used by this experiment to both treatments. It adds no
logical axiom. `augment.py` applies the same preparation before adding the
single induction schema.

Read `PREREGISTRATION.md` before running the experiment. Raw results belong
under `.artifacts/experiments/2026-07-29-022-integer-induction-schema/`.
