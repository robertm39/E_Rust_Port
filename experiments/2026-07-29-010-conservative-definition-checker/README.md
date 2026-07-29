# Conservative-definition proof checking

Bead: `E_Rust_Port-9jt.2.10`

This experiment evaluates an external validation path for first-order TSTP
proofs whose dependency closure contains a conservative predicate definition.
It keeps Umlaut's proof object unchanged and compares:

- ProofCheck 1.0, retained as the negative coverage control; and
- ProofGuard 1.0 at a pinned Git revision, whose independent checker validates
  fresh, non-circular predicate definitions before semantically replaying
  every dependent inference with E.

ProofGuard is used only as a caller-supplied external process. Its upstream
repository has no license declaration at the pinned revision, so none of its
source or binaries are copied into Umlaut, its packages, or this experiment.

See `PREREGISTRATION.md` for the frozen gate and `FINDINGS.md` for the passing
result. Results remain in ignored artifact storage.
