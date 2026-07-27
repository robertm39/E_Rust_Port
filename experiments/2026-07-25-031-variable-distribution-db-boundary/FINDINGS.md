# Experiment 332: variable-distribution DB boundary

## Status

Completed for Bead `E_Rust_Port-j76.3.486`.

## Question

Does Rust retain C `TermAddVarDistribution`'s positive-f-code assertion for a
non-free DB-variable term now that higher-order feature extraction is complete?

## Baseline

- C tests `TermIsFreeVar` and asserts `term->f_code > 0` for every other term.
- The first requested DB variable has f-code zero, is not an ordinary free
  variable, and therefore trips that C assertion.
- Rust already has the same precondition and diagnostic, but no regression
  constructed the typed DB-variable shape directly.

## Candidate

Retain the existing C-compatible assertion and add a direct typed DB0
regression. Do not reinterpret DB variables as ordinary variable-distribution
entries.

## Exact commands

Validation runs on an ephemeral native-Linux worker:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && bash experiments/2026-07-25-031-variable-distribution-db-boundary/remote_validate.sh"
.\linode-runner.ps1 down
```

## Results

- Worker `e-rust-codex-260726-181019-c84e` validated source snapshot
  `a16d03fa52e08884bf1795a254503ba3ebcbc96f085f4887bdda836583c4cdfa`.
- `cargo fmt --all -- --check` passed.
- Strict all-target/all-feature Clippy passed.
- The exact DB0 assertion regression passed (`1 passed`, `4402 filtered
  out`).
- The complete all-target/all-feature suite passed: `4,410` library tests
  plus `11` integration tests (`4,421 passed`, zero failures).
- The worker and its firewall were deleted after validation.

## Falsification rule

Reject the retained boundary if DB0 is counted as a free variable or traversed
without the C assertion, or if formatting, strict Clippy, or any Rust test
fails.

## Conclusion

The existing boundary is retained. DB0 remains a non-free term with f-code zero
and triggers the same positive-code assertion as C instead of being counted as
an ordinary variable occurrence. The former “higher-order feature extraction
remains incomplete” qualifier is obsolete; the low-level compatibility
precondition itself is now directly pinned.

## Limits

- This records the low-level C helper contract; it does not assert that every
  higher-order feature owner invokes this first-order distribution helper.
- C was not modified.
