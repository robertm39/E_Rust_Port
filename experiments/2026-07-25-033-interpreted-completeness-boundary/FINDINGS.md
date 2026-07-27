# Experiment 334: interpreted-symbol completeness boundary

## Status

Completed for Beads `E_Rust_Port-j76.3.219` and
`E_Rust_Port-j76.4.1191`.

## Question

Does interpreted arithmetic remain missing drop-in behavior, or does unchanged
C deliberately classify every external interpreted `$identifier` as
unimplemented and report restricted-calculus exhaustion?

## Baseline

- C `TermSigInsert` assigns `FPInterpreted` to external `$identifier` terms.
- C `SigHasUnimplementedInterpretedSymbols` returns true for any external
  symbol carrying that property; it has no implemented-arithmetic exception.
- C's main proof-search completeness gate clears `inf_sys_complete` when that
  helper returns true.
- Rust mirrors all three stages and has a permanent executable regression for
  an external `$foo` term.

## Candidate

Retain the C boundary. Do not implement an arithmetic evaluator that would make
Rust claim completeness where this C revision reports `GaveUp`.

## Exact commands

Validation runs both freshly built executables on native Linux:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && bash experiments/2026-07-25-033-interpreted-completeness-boundary/remote_compare.sh"
.\linode-runner.ps1 down
```

The script copies the immutable C checkout to temporary storage before
configuring/building it, builds the Rust release executable, and compares the
exit code, stderr, restricted-calculus completion line, and SZS status.

## Results

- Worker `e-rust-codex-260726-182042-0a22` validated source snapshot
  `16e75ebff3affd15f07c076f5d0465469f52af68ec72965d9d229cb2e9656e53`.
- Two setup attempts stopped before comparison because direct copies of the
  Windows checkout retained non-executable/CRLF build scripts. The retained
  compatibility builder's `prepare_reference_source` helper fixed the
  disposable C build tree without touching `eprover/`.
- The final native FOL C and Rust release runs matched exactly:
  - exit code `10`;
  - empty stderr;
  - `% Clause set closed under restricted calculus!`;
  - `% SZS status GaveUp`; and
  - byte-identical complete stdout, SHA-256
    `7796f8bd44934eb469cb700a7678838e38cf3caff1e33befbd099bbd69dc206c`.
- The retained machine-readable result is [`reference.json`](reference.json).
- Experiment 332 already passed strict formatting/Clippy and all `4,421`
  current Rust tests, including the permanent interpreted-symbol executable
  regression.
- The worker and firewall were deleted after comparison.

## Falsification rule

Reject the compatibility decision if unchanged C has an implemented external
arithmetic-symbol exception, if C and Rust disagree on exit/status/completion
for the retained interpreted fixture, or if either executable emits an
unexpected diagnostic.

## Conclusion

The candidate is retained. “Interpreted arithmetic remains later work” was a
stale scope claim: this C revision marks every external interpreted symbol as
unimplemented and deliberately refuses to report satisfiability after
restricted-calculus exhaustion. Adding arithmetic evaluation would exceed,
rather than complete, drop-in behavior.

## Limits

- This slice audits the completeness contract, not parsing of every TPTP
  arithmetic spelling.
- Internal `$true`, `$false`, and answer symbols are below C's external-symbol
  scan boundary and remain unaffected.
- C was not modified.
