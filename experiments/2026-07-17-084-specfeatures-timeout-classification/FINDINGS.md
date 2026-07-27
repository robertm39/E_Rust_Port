# Spec-feature timeout classification

## Status

Completed for Bead `E_Rust_Port-j76.2.53`. The timeout-bounded CNF
classification path was already present, but focused unchanged-C comparison
found and fixed its zero-timeout fast-child boundary. The vendored C checkout
remained unchanged.

## Existing computation and process boundary

C `ClausifyAndClassifyWTimeout` forks the already parsed and SInE-filtered
proof state, applies the requested soft CPU limit, runs formula conjecture
preprocessing and CNF, computes `SpecFeaturesCompute` plus
`SpecFeaturesAddEval`, and writes the fixed 22-byte class buffer. The parent
substitutes 21 hyphens only when it reads a short buffer.

Rust already represented the same deterministic work through the hidden
`classify_problem` re-exec child. It reparses the static input snapshot, reapplies
SInE, uses the C merged-CNF constants, calls the full spec-feature wrapper, and
returns the same fixed-width buffer. The parent uses a portable elapsed-time
guard because native Windows has neither `fork()` nor `RLIMIT_CPU`.

## Zero-timeout mismatch and fix

Rust previously returned the hyphen fallback before spawning a child whenever
the timeout was zero. That was stricter than C. A POSIX zero CPU limit is
delivered asynchronously; a sufficiently fast forked child can compute and
write its class before the first accounting signal. On the isolated reference,
the small CNF fixture completed 40 out of 40 zero-timeout probes.

Rust now always starts the child. A zero timeout receives a bounded 100 ms
re-exec grace so native process startup does not erase the fast-child outcome;
non-completing work is still killed and converted to the same short-buffer
fallback. The focused regression now expects a complete tiny-input class and
separately pins the bounded grace. Short child output remains independently
covered by the existing 21-hyphen regression.

## Executable evidence

[`compare_timeout_classification.py`](compare_timeout_classification.py)
compares complete stdout, stderr, and exit status for:

- completed CNF, FOF, and THF children with a 10-second limit;
- the fast-child zero-timeout boundary;
- the `-1` merged-mode disable sentinel; and
- the normal-Linux effectively unbounded `-2` limit.

All six cases are byte-exact between unchanged C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the optimized Rust
executable. The five merged outputs are 50 bytes; the disabled `-1` case takes
the standard classification path and emits 195 bytes.

[`reference.json`](reference.json) retains the 6/6 result and has SHA-256
`7AF7751D2A9A27351970A8D8F49E020F46BC87355630CE7580B88F62F10794A0`.

## Compatibility decision

The stable feature computation and fixed-buffer contract are complete. Exact
CPU-time scheduling at the zero boundary is inherently host-dependent, and
Rust retains the documented portable re-exec/wall-time implementation instead
of introducing a Unix-only process model. The bounded grace preserves the
observed fast-child behavior without making zero-timeout work unbounded.

## Reproduction

```powershell
cargo build --locked --release --bin classify_problem --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-084-specfeatures-timeout-classification\compare_timeout_classification.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/classify_problem `
  --rust-exe target\release\classify_problem.exe `
  --output target\specfeatures-timeout-reference.json `
  --expected experiments\2026-07-17-084-specfeatures-timeout-classification\reference.json
```
