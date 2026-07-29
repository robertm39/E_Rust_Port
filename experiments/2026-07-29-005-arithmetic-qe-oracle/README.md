# Independent arithmetic and QE oracle

Bead: `E_Rust_Port-9jt.5.5`

This experiment exercises the reusable, Umlaut-independent exact oracle in
`tools/validation/arithmetic_qe_oracle.py` against a pinned external Z3
process. It covers exact rational helpers, bounded real cell decomposition,
bounded integer enumeration, known VIRAS paper errata, deliberately faulty
transformations, deterministic shrinking, metamorphic properties, and
generated differential queries.

The experiment is clean-room work based on the tracked `viras_docs/` research
packet. It does not inspect or use the unlicensed VIRAS implementation. Z3 is
an ignored reference input and remains absent from Umlaut, Cargo, and every
package.

Run from the repository root:

```text
python experiments/2026-07-29-005-arithmetic-qe-oracle/run_experiment.py \
  --z3 /absolute/path/to/pinned/z3 \
  --seed 0x5A172026 \
  --cases 500 \
  --output .artifacts/experiments/2026-07-29-005-arithmetic-qe-oracle/report.json
```

See `FINDINGS.md` for the pinned revision, remote build/run procedure,
outcomes, falsification checks, retained evidence, and limits.
