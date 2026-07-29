# TSTP input-leaf provenance repair

Bead: `E_Rust_Port-9jt.2.9`

This experiment preserves and repairs the first-order proof-output failures
exposed by the bounded AVATAR study. The production changes are evaluated
against four independently visible boundaries:

- source CNF bodies whose variables or literal order are canonicalized;
- source `negated_conjecture` CNF bodies with mixed literal signs;
- definitional-CNF symbols that require conservative TSTP introduction
  metadata; and
- proof objects interrupted after a successful search claim.

The source fixtures are deliberately small. The large held-out representatives
remain `COL003-19`, `SYN846-1`, `PUZ008-2`, and `GRP667-4` from the frozen
CASC-30 corpus. ProofCheck is used only from its pinned, self-certified binary
bundle on the Ubuntu runner.

See `PREREGISTRATION.md` for the frozen gates and `FINDINGS.md` for the final
result. Source-leaf and atomic-publication gates passed. Used conservative
definitions remain an explicit ProofCheck 1.0 coverage gap tracked by
`E_Rust_Port-9jt.2.10`; they are not counted as verified.
