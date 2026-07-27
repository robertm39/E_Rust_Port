# Proof-output integration parity

Date: 2026-07-18

Bead: `E_Rust_Port-j76.2.33`

Upstream reference commit: `17026b1bfe61aaf223cfaae54947c8d2679c31a0`

## Question

Do the production owners for `--proof-object`, `--proof-graph`,
`--proof-statistics`, `--full-deriv`, `--force-deriv`, `--record-gcs`, and
`--training-examples` preserve upstream proof extraction, ordering,
renumbering, graph rendering, proof marking, and training behavior?

## Method

`compare_proof_outputs.py` runs 15 production CLI cases against the Rust
release binary and the isolated upstream higher-order executable. It compares
exit codes, stdout, and stderr byte-for-byte after normalizing only CRLF to LF.
The fixtures exercise a mixed formula/clause refutation with an irrelevant
formula, a clause-only refutation, and a satisfiable formula.

The matrix covers TSTP and PCL proof lists, full derivations, graph levels 1
and 2, graph output with recorded given clauses, proof statistics with and
without a list, positive and negative training examples, saturation lists and
graphs, forced derivation levels 1 and 2, and `--proof-object=0` suppression.

The retained compact reference is `reference.json`; its SHA-256 is
`eb476294f3c6a9a2488e0a68e92524920157f3c08bebf26cb194706c923e0b0e`.

Reproduction from the repository root:

```powershell
cargo build --locked --release --bin eprover --all-features
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe experiments\2026-07-18-105-proof-output-integration\compare_proof_outputs.py --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho --rust-exe target\release\eprover.exe --output target\proof-output-comparison.json --expected experiments\2026-07-18-105-proof-output-integration\reference.json
```

## Findings

The first production comparison found five connected compatibility defects:

1. DOT edges were emitted after all nodes instead of immediately after each
   child, and the two possible integer encodings of a Rust rewrite-demodulator
   handle could resolve to the same owner and duplicate an edge.
2. Detailed DOT derivations printed numeric formula-parent display IDs where C
   retained the input formula names.
3. Processed `evalgc` nodes were boxes instead of C's ellipses, and saturation
   graph nodes were colored as proof members instead of gray. Rust now computes
   C's proof-membership closure for graph rendering without mutating stable
   proof-state identities.
4. Proof marking ignored rewrite-demodulator parents. This mislabeled selected
   clauses used through rewriting as negative training examples.
5. Training output used the clauses' stable internal IDs instead of the
   display-only renumbering created by C's `DerivationRenumber`, and missed C's
   literal doubled-percent training suffix. An ephemeral owner-to-display-ID
   map now reproduces that output while leaving internal generation-aware
   references unchanged.

The final matrix is 15/15 exact, and every independent effect assertion is
true.

## Ownership decision

The legacy note about adding production formula extraction roots is not an
upstream compatibility requirement. C's `ProofState` has one
`extract_roots` stack of clauses, `DerivationCompute` accepts a clause-root
stack, and `DerivationExtract` discovers formulas only through formula-parent
edges. Rust's direct formula-root support is a safe extension used by focused
tests, but production formula-producing call sites must not invent roots that C
does not select.

The other legacy residuals are now represented by durable owners:

- `ProofObjectGraph::c_ordered_nodes` performs C's mixed formula/clause
  topological order and display-only renumbering;
- list and DOT writers consume that same order and exact borrowed owners;
- formula archives are resolved through the formula-parent references selected
  by clause-root extraction rather than independent archive roots; and
- clause marking resolves generation-aware clause and demodulator references,
  avoiding persistent raw-pointer identity while preserving C owner selection.

## Validation

- focused DOT, demodulator-resolution, proof-marking, and training regressions:
  passed;
- release `eprover` build: passed;
- retained production C/Rust matrix: 15/15 exact;
- full all-target/all-feature suite: 4,321 library tests plus every binary and
  integration target passed;
- strict all-target/all-feature pedantic Clippy: passed; and
- formatting and all four C-source documentation integrity gates: passed.

The vendored `eprover/` tree was not modified.
