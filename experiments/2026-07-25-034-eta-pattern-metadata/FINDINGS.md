# Experiment 335: Eta-normalized applied-variable metadata

## Status

Completed for Beads `E_Rust_Port-j76.3.294`,
`E_Rust_Port-j76.3.644`, `E_Rust_Port-j76.4.183`, and
`E_Rust_Port-j76.4.1286`.

## Question

Does term-bank insertion reproduce C `NormalizePatternAppVar` by
eta-reducing applied free variables before assigning pattern metadata, cached
counts, and standard weight, and does extensional indexing consume that
metadata correctly?

## Baseline

- Rust recognized applied free variables whose existing visible arguments were
  distinct DB variables.
- The higher-order pattern matcher already had a private exact port of
  `NormalizePatternAppVar`.
- Term-bank metadata did not run eta reduction, so an argument such as
  `lambda x. db(1) x`, whose eta normal form is `db(0)`, left the enclosing
  applied free variable marked non-pattern.
- Extensional indexing intentionally uses the term-bank pattern flag as the
  truth value corresponding to C `MAYBE_NORMALIZE_APP_VAR`, so the stale flag
  also changed index traversal.

## Candidate

Move the shared normalizer into the lambda module and call it while assigning
new term-bank metadata, after the original cell has been shared in the bank as
in C. Preserve the original applied term while using the normalized result to
choose its pattern flag, variable/function counts, and standard weight.

Two direct regressions build a loose-DB eta redex and pin both:

- pattern metadata and single-variable cached measures; and
- omission of the applied variable from extensional-into positions.

## Exact commands

Validation runs on an ephemeral native-Linux worker:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && bash experiments/2026-07-25-034-eta-pattern-metadata/remote_validate.sh"
.\linode-runner.ps1 down
```

## Results

- The ephemeral worker
  (`e-rust-codex-260726-184054-b6fc`) required a manual continuation of the
  standard bootstrap after the local controller timed out during provisioning;
  the worker and validation environment otherwise matched the repository
  runner definition.
- The first candidate run passed formatting and strict Clippy. Its focused
  term-bank regression passed, while the extensional-index fixture failed
  before reaching the assertion because it supplied an unshared arrow type to
  the DB-variable bank. Interning that test type through the bank fixed the
  falsified fixture without changing production code.
- The final source snapshot
  (`21a4e64c9906577e5f7ae74ee8765a7e4f83b7b0cabeb13863093512b4091d0c`)
  passed:
  - `cargo fmt --all -- --check`;
  - strict all-target/all-feature Clippy;
  - both focused eta-normalization regressions; and
  - the complete all-target/all-feature suite (`4,412` library tests plus
    `11` integration tests, `4,423 passed`, zero failures).
- The worker and its firewall were deleted after validation.

## Falsification rule

Reject the candidate if the loose-DB eta redex does not normalize to a valid
pattern argument, if the original applied term is not counted as one variable,
if extensional indexing descends below it, or if strict formatting, Clippy, or
the complete Rust suite fails.

## Conclusion

The candidate is retained. Rust now exposes one exact
`NormalizePatternAppVar` implementation, calls it from higher-order pattern
matching and term-bank metadata assignment, and applies C's single-variable
counts and weight after eta normalization. Because `ccl_ext_index` uses the
normalization result only as a truth value, its metadata-based traversal now
also matches C for eta-normalizable applied variables.

## Limits

- This slice restores the exact term-bank normalization decision and its
  extensional-index consumer; it does not add C's per-term owner backpointer or
  binding/WHNF cache.
- C is not modified.
