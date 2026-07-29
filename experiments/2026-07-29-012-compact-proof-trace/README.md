# Compact proof-trace storage

This experiment supports Bead `E_Rust_Port-9jt.8.2`. It evaluates a
deterministic framed proof-output log, exact reconstruction, independent
checking, spooling, and fail-closed recovery before deciding whether any live
search representation should change.

The frozen design and decision rules are in
[`PREREGISTRATION.md`](PREREGISTRATION.md). Raw proofs, checker binaries,
profiles, timing samples, and malformed logs are retained outside Git.

No prover or checker command may run locally. Controller tests may run locally
after the harness is added; all empirical work uses the Ubuntu runner.
