# LUSK6ext rewrite-link requeue trace

## Question

Why does C backward-rewrite clause 680 with the newly selected unorientable
demodulator 2574 before the ordinal-64 selection, while Rust later selects the
uncontracted corresponding clause?

## Setup and exact commands

The reference was E 3.3.5 at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` from the `e-interop` WSL cache.
The Rust candidate was the Windows release build on
`codex/initial-rust-port-slice`.

Representative commands, run from the repository root:

```powershell
cargo build --locked --release --bin eprover
.\target\release\eprover.exe --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6ext.lop
cargo test compute_all_paramodulants_with_docs_prints_plain_creation_step
cargo test indexed_plain_paramodulation_preserves_c_variable_normalization_order
cargo test indexed_forward_rewrite_matches_lusk6ext_clause_680_root
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
```

```bash
gdb -q -batch -x experiments/2026-07-12-003-rewrite-link-requeue/trace-c-requeue.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
gdb -q -batch -x experiments/2026-07-12-003-rewrite-link-requeue/trace-c-forward-chain.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
gdb -q -batch -x experiments/2026-07-12-003-rewrite-link-requeue/trace-c-forward-order.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
gdb -q -batch -x experiments/2026-07-12-003-rewrite-link-requeue/trace-c-earlier-bwrw.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
```

Debugger scripts are retained in this experiment folder. Raw outputs are under
`.artifacts/e-compare/rewrite-link-requeue/`; the full post-fix differential
report is `.artifacts/e-compare/20260712-093742-395646/`, and the paired
benchmark is `.artifacts/e-compare/20260712-101859-557225-benchmark/`.

## Key results

- C does not backward-rewrite queued clause 680 when clause 712 is selected:
  `trace-c-earlier-bwrw.gdb` reports zero candidates and clause 680 is not yet in
  the processed rewrite index.
- C likewise finds zero backward-rewrite candidates when clause 2574 is selected.
  Clause 680 is instead rewritten later during its own forward contraction.
- The accepted C forward match uses clause 2574 variables
  `left=[-2,-4,-6]`, `right=[-6,-4,-2]`, binds `-2` to a unary compound,
  `-4` to variable `-2`, and `-6` to variable `-4`, receives KBO6 result
  `to_greater`, and installs the rewrite link. The focused Rust rewrite test
  produces the same replacement.
- C and Rust both generate the exact pre-rewrite clause and the transformed
  equivalent. Rust evaluates the exact clauses ahead of the transformed clauses,
  so HCB queue ordering is not the source of the observed selection difference.
- Source review exposed a separate observable C behavior: unindexed
  `ComputeClauseClauseParamodulants` pushes derivations onto its temporary
  selected-clause copy rather than the generated child. Rust now leaves those
  unindexed children without a generating derivation, while indexed children
  retain normal derivations.
- The 50-case report has 7 mismatches, down from 8 because `GEO288+1.p` now
  matches. `LUSK6ext.lop` remains a normalized-output mismatch, and its retained
  proof diff is byte-for-byte unchanged from the preceding report.
- The five-run benchmark reports a `3.641x` aggregate Rust/C ratio.
  `LUSK6ext.lop` measures `3.211x`; all nine behavior-matching cases exceed the
  required `1.10x` threshold, while `BOO020-1.p` is excluded for differing
  outcomes.

## Falsification checks

- The backward-requeue hypothesis was rejected by both C candidate-stack probes:
  each reported `candidate count=0`.
- The forward-rewrite implementation hypothesis was rejected by the exact Rust
  clause-680 regression and the C substitution/KBO trace.
- The generation hypothesis was rejected because both provers emit both relevant
  clause shapes.
- The evaluation-order hypothesis was rejected because Rust assigns every exact
  form a lower evaluation than the transformed form.
- Removing the unindexed child derivation was not credited as a LUSK6ext fix:
  the new normalized proof is unchanged even though the overall mismatch count
  improves.
- The nested `eprover/` checkout was never modified; all C probes are debugger
  scripts and ignored artifacts outside that tree.

## Conclusion and limits

Clause 680's C rewrite is an ordinary forward contraction using a valid
shared-variable match against clause 2574. Rust already supports that exact
match, and neither backward indexing nor HCB evaluation order explains the
remaining LUSK6ext retained-proof difference. The next LUSK6ext investigation
should start at the earliest retained-proof delta: Rust retains an extra clause
`f(X1,f(X1,f(X1,g(X1)))) = g(X1)` before the common `j/f/g` clause, rather than
continuing from the later ordinal-64 trace.

The experiment did produce one compatibility fix. C's apparent unindexed
paramodulation derivation-target typo affects orphan filtering and therefore
proof search; preserving it restores exact `GEO288+1.p` compatibility. It is
documented as a Change Later candidate rather than silently corrected in Rust.
