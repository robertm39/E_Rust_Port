# Eligible branching-CSU equality resolution

## Status

Completed for Bead `E_Rust_Port-j76.2.112`. The previously pending broad
multi-CSU coverage now includes an eligible equality-resolution literal that
produces two observably different resolvents. The vendored C source remains
unchanged.

## Missing case

The earlier external trace in
`experiments/2026-07-15-002-equality-resolution-multicsu/` proved that C and
Rust agree on higher-order ordering initialization and correctly reject its
flexible-rigid disequality before CSU enumeration. That was important
production coverage, but its equality-resolution count was deliberately zero.

The new source-shaped regression resolves `F(a)!=a` and retains the positive
literal `F(b)=e`. With pattern and fixpoint oracles disabled and one imitation
plus one functional projection enabled, the two CSU branches remain visible:

- projection binds `F` to the identity and produces `b=e`; and
- imitation binds `F` to the constant function returning `a` and produces
  `a=e`.

## Ordering and metadata

C `ComputeEqRes` pushes each CSU resolvent in enumeration order and its caller
later pops that result stack. Rust's temporary vector follows the same shape.
The focused regression therefore requires projection-derived `b=e` first and
imitation-derived `a=e` second, with proof identifiers 1 and 2 and exact
`er(45)` PCL records in that order.

C ORs `subst_is_ho` across every CSU result for the selected literal before
the caller drains the stack. The regression also requires both resolvents to
carry the higher-order `DCEqRes` operation, including the parent reference.
This distinguishes aggregate compatibility from a per-result flag redesign.

## External comparison status

`trace.sh` and `input.p` prepare a direct C/Rust executable comparison that
normalizes only clause identifiers and checks two `inference(er,...)` records
plus the equality-resolution statistics. The reconciliation session could not
run it because `wsl --list --quiet` exposed no distribution for the active
Windows user; invoking the configured `Ubuntu-24.04` name returned
`WSL_E_DISTRO_NOT_FOUND`.

The expected stack reversal is independently supported by the unchanged C
source and by the 2026-07-15 equality-factor executable trace, which exercises
the same C CSU enumeration and result-stack drain with the same imitation and
projection shapes. This limitation is recorded rather than claiming an
unobserved executable match.

## Validation

- the new eligible branching-CSU regression passes;
- all 17 focused `eqnresolution` tests pass;
- all 4,229 library tests plus every integration and binary target test pass;
- formatting and strict Clippy pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
