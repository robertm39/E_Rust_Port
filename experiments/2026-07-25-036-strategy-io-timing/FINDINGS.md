# Strategy-I/O timing

## Status

Completed for Beads `E_Rust_Port-j76.3.98` and
`E_Rust_Port-j76.4.1171`. Rust now reaches `--print-strategy` through the same
post-CNF strategy-I/O boundary used by ordinary proof search. The vendored C
checkout remains unchanged.

## Discrepancy

C parses the input, performs automatic preprocessing selection, SInE and
relevance filtering, formula CNF, clausal preprocessing, and optional search
selection before `strategy_io`. A requested strategy print exits only after
the parsed strategy file and selected named strategy have been applied.

Rust already applied `--parse-strategy` and `--select-strategy` at that
boundary for proof search, but a separate top-level `--print-strategy` branch
printed before allocating a proof state or opening an input. It manufactured
the empty BCE and predicate-elimination lines needed by earlier output tests
instead of obtaining them from the real preprocessing passes. It also gave
strategy printing precedence over C's earlier syntax-only, app-encode, and
prune-only exits.

## Retained implementation

The early branch is removed. `run_proof_search` now applies the shared
strategy-I/O parse/select operation after preprocessing and calls a small
render-only helper at that exact point. The helper handles the four C print
requests: all predefined strategies, all names, current parameters, or a
named predefined strategy.

Consequently:

- malformed problem input fails before any strategy is printed;
- preprocessing output is produced by the real enabled passes;
- strategy-file warnings and selected-strategy errors keep their order;
- syntax-only, app-encode, and prune-only retain their earlier C exits; and
- ordinary runs without `--print-strategy` retain their existing proof-control
  initialization path.

## Exact C/Rust comparison

[`compare_strategy_timing.py`](compare_strategy_timing.py) compares the
unchanged C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` with the optimized Rust
executable. [`reference.json`](reference.json) pins all exit codes, stream
sizes, and SHA-256 digests.

All three cases are byte-exact:

| Case | Boundary | Exit | Stdout | Stderr |
| --- | --- | ---: | ---: | ---: |
| `post_cnf_print` | valid FOF reaches post-CNF strategy print | 0 | 4,569 bytes | empty |
| `syntax_only_precedence` | syntax-only exits before strategy I/O | 0 | 44 bytes | empty |
| `invalid_input_precedes_strategy` | invalid TCF fails during parsing | 3 | empty | 167 bytes |

The post-CNF strategy cell has SHA-256
`105adb60bc456451d5c06142a7180122544d989c5944fbf1277f9593ca9d6aca`
in both implementations.

## Reproduction

Run on the required ephemeral Ubuntu worker:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo build --locked --release --bin eprover && python3 experiments/2026-07-25-036-strategy-io-timing/compare_strategy_timing.py --repo . --c-exe /root/.cache/e-rust-port/bin/worktree-snapshot/fol/eprover --rust-exe target/release/eprover"
}
finally {
    .\linode-runner.ps1 down
}
```

Build the pinned reference first with `linux_compat.py build-reference` when
the worker cache is empty.

## Validation

Eight existing strategy-print regressions and two new ordering regressions
pass. The new tests pin parsing-before-print and syntax-only-before-strategy
precedence. The final source snapshot
`341962a30a7cc815f467c747f14045824089456913a15d4206f43a4e10938106`
on ephemeral worker `e-rust-codex-260726-191044-7524` passed:

- `cargo fmt --all -- --check`;
- strict all-target/all-feature pedantic Clippy;
- the complete all-target/all-feature suite (4,414 library tests plus 11
  integration tests, 4,425 total); and
- the optimized three-case exact C/Rust comparison.
