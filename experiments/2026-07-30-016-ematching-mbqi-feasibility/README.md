# E-matching and MBQI feasibility

This experiment evaluates Bead `E_Rust_Port-9jt.6.6` without changing
production behavior.

The frozen protocol is in [PREREGISTRATION.md](PREREGISTRATION.md). It compares
bounded complete clausification, deterministic unary/multipattern E-matching,
and model-counterexample instantiation on the exact EPR corpus preserved by
Experiment 008. Every terminal answer is fail-closed and backed by replayable
source substitutions plus either a checked DRAT proof or an exhaustive finite
model check.

The completed result is in [FINDINGS.md](FINDINGS.md). Every certificate
validated, but E-matching added no held-out solve, lost one MBQI solve, used
2.05 times MBQI's instances on common solves, and produced nine
repeat-unstable time-limited traces. The verdict is `stop`; production remains
unchanged.
