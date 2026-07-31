# IPASIR-UP-style theory propagation

This experiment evaluates Bead `E_Rust_Port-9jt.4.6` without changing
production behavior.

The frozen protocol is in [PREREGISTRATION.md](PREREGISTRATION.md). The
simulation compares final-model-only theory conflicts, partial theory
conflicts, eager theory propagation, and a fully encoded reference on a
source-backed pigeonhole split. Every external clause and root backtrack is
independently replayed.

The completed result is in [FINDINGS.md](FINDINGS.md). Correctness and
determinism passed, but propagation failed the frozen search-reduction gate and
production remains unchanged.

Run the focused tests with:

```text
python3 -m unittest discover \
  -s experiments/2026-07-30-015-ipasir-up-propagation \
  -p 'test_*.py' -v
```
