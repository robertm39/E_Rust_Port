# Set-level forward-contraction proof-documentation owners

## Status

Completed for Bead `E_Rust_Port-j76.2.49`. Rust now preserves C's shared
proof-documentation session across set-level forward contraction and the
post-saturation `ProofStateFilterUnprocessed` executable owner. The vendored C
checkout remained unchanged.

## Ownership result

C's `ForwardContractSet` calls the same static `forward_contract_keep` helper
as `ForwardContractClause`, so its rewrite, minimization, condensation, and
simplify-reflect proof side effects use the process-global `DocOut` stream.
`ProofStateFilterUnprocessed` routes every contracting descriptor through that
set helper, and `eprover` invokes the filter after saturation.

Rust previously represented the local documenting helper only for selected
clauses. The set wrapper always selected the plain path, and the executable
dropped its saturation `ProofDocSession` before filtering. The new
`proof_state_forward_contract_set_with_docs` and
`proof_state_filter_unprocessed_with_docs` wrappers share one session through
every requested filter operation. `eprover` creates a successor session at the
identifier returned by saturation and advances the identifier again before
printing saturated-set and proof-result side outputs. Output levels below two
continue to use the plain path.

The focused set regression creates a real minimization modification and checks
its represented PCL event. The executable regression stops after selecting one
demodulator, lets full post-saturation contraction rewrite a queued non-unit
clause into duplicate literals, and verifies that minimization receives
identifier `c_0_5`. The later saturated-set quote receives `c_0_7` (after the
processed-set quote at `c_0_6`) and names `c_0_5` as its parent, proving session
continuity across the phase boundary.

## Reference-CLI constraint

The unchanged C option table documents `--filter-saturated` as an optional
argument defaulting to `Fc`, and `ProofStateFilterUnprocessed` implements
`ucnNrRfF`. Its `eprover.c` validation nevertheless accepts the unrelated
print-set alphabet `eigEIGaA`; the cached C executable consequently rejects
both the bare default and an explicit `F`. Rust retains that observable CLI
behavior for drop-in compatibility. The focused executable regression therefore
constructs the already-parsed `EProverConfig` internally, while the C/Rust
ownership relationship is pinned by the source audit.

[`audit_forward_contract_set_docs.py`](audit_forward_contract_set_docs.py)
checks all 12 C/Rust call-graph, session-lifetime, and regression contracts.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-088-forward-contract-set-doc-owners\audit_forward_contract_set_docs.py `
  --repo . `
  --output target\forward-contract-set-doc-audit.json `
  --expected experiments\2026-07-17-088-forward-contract-set-doc-owners\audit-reference.json

cargo test --locked proof_state_forward_contract_set_with_docs_emits_modification_step
cargo test --locked run_config_filter_saturated_continues_proof_doc_session
```

## Validation

- source/test ownership audit: 12/12 contracts passed;
- focused set-level modification and executable session-continuity regressions
  passed;
- full all-target/all-feature suite, strict pedantic Clippy, formatting,
  documentation integrity gates, and optimized executable build are recorded in
  the completing commit; and
- the vendored C worktree remained clean.
