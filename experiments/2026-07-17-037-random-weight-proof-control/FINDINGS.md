# Random-weight proof-control integration

## Objective

Resolve `E_Rust_Port-j76.2.100` by checking whether the existing `che_random` port reaches the live proof-search heuristic path and by adding deterministic reference coverage for random-dependent selection. The vendored C source remains unchanged.

## Reference trace

C `ProofControlInit` parses command-line and strategy WFCB definitions before HCB definitions, installs the named active HCB, and uses `HCBClauseEvaluate` for both initial and generated clauses. `RandWeightCompute` consumes the old FIFO counter and calls `JKISSRandDouble(&(local->rand_state))`; the helper ignores that pointer and advances its file-static generator words. With the default global state, C's first two unsigned results are `560241513` and `2602615593`.

Rust already matched every production link: option definitions feed `proof_control_init_heuristics`, `RandomWeight` dispatches through `WfcbAdmin`, HCB evaluation attaches indexed clause evaluations, and proof-control selection extracts the best evaluation. The missing part was one regression spanning those links.

## Regression

The new proof-control test installs `random_eval=RandomWeight(ConstPrio,1000,0,0,11,13,17)` and `RandomEvalStoreTest=(1*random_eval)`, then evaluates two generated-clause queue entries through the active HCB. The nonzero seeds intentionally demonstrate the C state-pointer quirk. The stored `float` bit patterns are pinned to `1124233471` and `1142390271`, the exact conversions of the first two C global JKISS results scaled by 1000. The evaluation index and live selection both choose the first clause.

All RNG-consuming unit tests involved in this surface now share the crate's global-state test lock and reset the JKISS state before asserting deterministic results. This changes test isolation only; production keeps C's process-global sequence.

## Compatibility decision

The migrated gap is complete rather than a missing integration. Per-evaluator seed cleanup remains intentionally deferred to `E_Rust_Port-j76.3.470`; negative optional-seed wrapping remains under `E_Rust_Port-j76.3.142`; the lower-level wrapper cleanup remains under `E_Rust_Port-j76.4.103` and `E_Rust_Port-j76.3.46`.

## Validation

- The focused live proof-control random-weight test, all four lower-level random-weight tests, and all four JKISS-related tests passed.
- 4,235 default-feature library tests passed.
- 4,240 all-feature library tests, every binary target, and all 7 integration tests passed.
- Strict all-target, all-feature pedantic Clippy passed.
- The release `eprover` build passed.
- `cargo fmt --all -- --check` passed.
- C-source documentation coverage, Change Later wording, Markdown-link integrity, and regeneration-preservation gates passed.
- The vendored `eprover/` worktree remained clean.
