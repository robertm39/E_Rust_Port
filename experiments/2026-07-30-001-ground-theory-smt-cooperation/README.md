# Cooperative ground-theory SMT experiment

Bead: `E_Rust_Port-9jt.5.7`

This experiment compares three treatments of a typed, ground arithmetic branch
stream:

1. no theory solver;
2. a persistent, shell-free Z3 SMT-LIB subprocess; and
3. a persistent Rust-to-Z3 C API driver.

The tracked corpus contains integer and real difference-logic branches,
deliberately unsupported linear branches, and neutral workloads. Z3 results are
never trusted directly. Unsatisfiable cores and satisfying models must pass an
independent exact Python verifier and an experiment-only Rust replay checker.
An unsupported or unverifiable result is `Unknown`, even when Z3 returns
`sat` or `unsat`.

The ignored Z3 checkout is pinned by commit and transferred separately to the
ephemeral Ubuntu runner. No Z3 source, binary, Cargo dependency, or production
integration is included in Umlaut.

Run the local, solver-free checks from the repository root:

```text
python experiments/2026-07-30-001-ground-theory-smt-cooperation/build_corpus.py --check
python -m unittest discover \
  -s experiments/2026-07-30-001-ground-theory-smt-cooperation \
  -p "test_*.py" -v
```

The live experiment requires the pinned Z3 executable, shared library, and the
two remotely compiled Rust drivers:

```text
python experiments/2026-07-30-001-ground-theory-smt-cooperation/run_experiment.py \
  --z3 /opt/e-rust-port/z3-build/z3 \
  --z3-library /opt/e-rust-port/z3-build/libz3.so \
  --z3-source-root /opt/e-rust-port/z3-src \
  --z3-source-archive /root/z3-2d48fd1.tar.gz \
  --z3-build-root /opt/e-rust-port/z3-build \
  --ffi-driver /opt/e-rust-port/ground-theory-z3-ffi \
  --replay-driver /opt/e-rust-port/ground-theory-replay \
  --output /opt/e-rust-port/ground-theory-smt-report.json \
  --certificate-output /opt/e-rust-port/ground-theory-certificates.txt
```

The exact runner commands, hashes, raw artifact paths, results, falsification
checks, and decision are recorded in `FINDINGS.md`.
